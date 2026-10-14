#!/usr/bin/env python3
"""Validate static content identity and derivatives, not runtime truth or UX."""
from html.parser import HTMLParser
from pathlib import Path
from xml.etree import ElementTree as ET
from datetime import datetime
from email.utils import parsedate_to_datetime
import sys
import re

ROOT = Path(__file__).resolve().parent.parent / 'site'

class Page(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.nodes = []
        self.text = []
        self.regions = {}
        self.stack = []
        self.feed(path.read_text(encoding='utf-8'))
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        self.nodes.append((tag, attrs))
        markers = [key for key in attrs if key in {'data-content-body','data-content-correction','data-content-description'}]
        if tag == 'h1': markers.append('h1')
        if tag not in {'meta','link','img','br','input','hr'}: self.stack.append((tag, markers))
    def handle_endtag(self, tag):
        for i in range(len(self.stack)-1,-1,-1):
            if self.stack[i][0] == tag:
                del self.stack[i:]; break
    def handle_data(self, data):
        self.text.append(data)
        for _,markers in self.stack:
            for marker in markers: self.regions.setdefault(marker,[]).append(data)
    def region(self, key):
        return ' '.join(''.join(self.regions.get(key,[])).split())
    def with_attr(self, key):
        return [(tag, a) for tag, a in self.nodes if key in a]

def check(root=ROOT):
    errors = []
    def need(condition, message):
        if not condition: errors.append(message)
    for prefix, locale in [('', 'zh-CN'), ('en/', 'en')]:
        home = Page(root / prefix / 'index.html')
        steps = home.with_attr('data-decision-panel')
        need([a['data-decision-panel'] for _, a in steps] == ['0','1','2','3'], f'{prefix}: four static reading steps required')
        need(all('hidden' not in a for _,a in steps), f'{prefix}: steps must remain readable without JS')
        walkthrough = home.with_attr('data-decision-walkthrough')
        need(len(walkthrough)==1 and walkthrough[0][1].get('data-provenance')=='illustrative', f'{prefix}: evidence type required')
        articles = []
        for path in sorted((root / prefix / 'devlog').glob('*/index.html')):
            page = Page(path)
            nodes = page.with_attr('data-content-id')
            need(len(nodes)==1, f'{path}: one canonical content identity required')
            if len(nodes)!=1: continue
            attrs = nodes[0][1]
            need(attrs.get('data-content-locale')==locale, f'{path}: wrong locale')
            need(attrs.get('data-content-kind')=='devlog', f'{path}: invalid content kind')
            need(attrs.get('data-content-status') in {'published','withdrawn'}, f'{path}: draft or unknown status in public route')
            need(attrs.get('data-content-id')==path.parent.name and re.fullmatch(r'[a-z0-9]+(?:-[a-z0-9]+)*',path.parent.name), f'{path}: invalid slug or id mismatch')
            need(attrs.get('data-translation-of')==path.parent.name, f'{path}: explicit translation identity missing')
            for field in ['data-published-at','data-updated-at']:
                try: datetime.strptime(attrs.get(field,''),'%Y-%m-%d')
                except ValueError: need(False,f'{path}: invalid {field}')
            need(bool(attrs.get('data-source-version')), f'{path}: source version missing')
            articles.append((path,attrs,page))
        feed = ET.parse(root / prefix / 'feed.xml').getroot()
        items=feed.findall('channel/item')
        need(len(items)==len(articles), f'{prefix}: RSS/detail set mismatch')
        ids = set()
        for path,attrs,page in articles:
            url=f'https://oasis7.tech/{prefix}devlog/{path.parent.name}/'
            item=next((i for i in items if i.findtext('guid')==url),None)
            need(item is not None,f'{path}: missing stable GUID')
            if item is None: continue
            need(url not in ids,f'{path}: duplicate GUID');ids.add(url)
            need(item.findtext('link')==url,f'{path}: feed target mismatch')
            need(item.findtext('title') == page.region('h1'),f'{path}: feed title drift')
            need(parsedate_to_datetime(item.findtext('pubDate')).date().isoformat()==attrs['data-published-at'],f'{path}: publication date drift')
            for surface in [root/prefix/'index.html',root/prefix/'devlog/index.html']:
                parsed=Page(surface)
                summaries=parsed.with_attr('data-content-summary')
                need(any(a['data-content-summary']==attrs['data-content-id'] for _,a in summaries) == (attrs['data-content-status']=='published'),f'{surface}: missing published derivative or withdrawn recommendation')
                if attrs['data-content-status']=='published': need(item.findtext('title') in ''.join(parsed.text),f'{surface}: summary title drift')
                if attrs['data-content-status']=='published': need(item.findtext('description') == parsed.region('data-content-description'),f'{surface}: summary description drift')
            if attrs['data-content-status']=='withdrawn':
                need(not page.with_attr('data-content-body'),f'{path}: withdrawn body must be removed')
                need(bool(page.region('data-content-correction')),f'{path}: explicit safe correction required')
                need(item.findtext('description') == page.region('data-content-correction'),f'{path}: withdrawn RSS must match correction')
            else:
                need(len(page.with_attr('data-content-body'))==1 and bool(page.region('data-content-body')),f'{path}: explicit published body required')
                need(item.findtext('description') == page.region('data-content-description'),f'{path}: canonical summary/RSS drift')
        need(len(ids)==len(items),f'{prefix}: unmatched RSS item')
        for family in ['observations','devlog']:
            need((root/prefix/family/'index.html').is_file(),f'{prefix}{family}: missing list')
        for _,a in home.with_attr('data-locale-en'):
            need('data-locale-zh' in a, f'{prefix}: missing locale map')
    zh_ids = {p.parent.name for p in (root/'devlog').glob('*/index.html')}
    en_ids = {p.parent.name for p in (root/'en/devlog').glob('*/index.html')}
    need(zh_ids == en_ids, 'content locale sets differ; untranslated content requires an explicit language fallback design')
    return errors

if __name__=='__main__':
    errors=check()
    if errors:
        print('\n'.join(errors),file=sys.stderr);sys.exit(1)
    print('site-discovery-check: OK (static steps, canonical content, paired summaries and RSS)')
