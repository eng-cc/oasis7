#!/usr/bin/env python3
"""Isolated negative publishing/parity cases for the static discovery gate."""
import importlib.util
from pathlib import Path
import shutil
import tempfile

spec=importlib.util.spec_from_file_location('discovery_check',Path(__file__).with_name('site-discovery-check.py'))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
files=['index.html','en/index.html','feed.xml','en/feed.xml','observations/index.html','en/observations/index.html','devlog/index.html','en/devlog/index.html','devlog/discovery-first/index.html','en/devlog/discovery-first/index.html']
cases=[
 ('index.html','data-decision-panel="3"','data-decision-panel="0"'),
 ('index.html','id="decision-2"','id="decision-2" hidden'),
 ('en/index.html','data-provenance="illustrative"','data-provenance="runtime_capture"'),
 ('devlog/discovery-first/index.html','data-content-status="published"','data-content-status="draft"'),
 ('devlog/discovery-first/index.html','data-content-status="published"','data-content-status="withdrawn"'),
 ('devlog/discovery-first/index.html','data-content-id="discovery-first"','data-content-id="different"'),
 ('devlog/discovery-first/index.html','data-source-version=','wrong-source-version='),
 ('devlog/discovery-first/index.html','data-published-at="2026-10-14"','data-published-at="invalid"'),
 ('en/devlog/discovery-first/index.html','data-content-locale="en"','data-content-locale="zh-CN"'),
 ('feed.xml','isPermaLink="true">https://oasis7.tech/devlog/discovery-first/','isPermaLink="true">https://oasis7.tech/devlog/missing/'),
 ('feed.xml','14 Oct 2026','13 Oct 2026'),
 ('index.html','data-content-summary="discovery-first"','data-content-summary="unrelated"'),
]
with tempfile.TemporaryDirectory() as tmp:
 root=Path(tmp)
 for file in files:
  target=root/file;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(module.ROOT/file,target)
 assert not module.check(root), module.check(root)
 for file,old,new in cases:
  target=root/file;source=target.read_text();assert old in source,(file,old)
  target.write_text(source.replace(old,new,1))
  assert module.check(root),(file,old,new)
  target.write_text(source)
with tempfile.TemporaryDirectory() as tmp:
 root=Path(tmp)
 for file in files:
  target=root/file;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(module.ROOT/file,target)
 # Withdrawal must not pass merely because homepage/list recommendations vanish.
 detail=root/'devlog/discovery-first/index.html'
 detail.write_text(detail.read_text().replace('data-content-status="published"','data-content-status="withdrawn"'))
 for rel in ['index.html','devlog/index.html']:
  p=root/rel;p.write_text(p.read_text().replace('data-content-summary="discovery-first"','data-content-summary="removed"'))
 errors=module.check(root)
 assert any('withdrawn body' in e for e in errors),errors
 assert any('withdrawn RSS' in e for e in errors),errors
 # A safe same-GUID correction with body and recommendations removed is accepted.
 import re
 for rel in ['index.html','devlog/index.html']:
  p=root/rel;p.write_text(re.sub(r'<article[^>]*data-content-summary="removed".*?</article>','',p.read_text(),flags=re.S))
 p=detail;s=p.read_text();s=re.sub(r'<article data-content-body>.*?</article>','<p data-content-correction>Record withdrawn; read the current project statement.</p>',s,flags=re.S);p.write_text(s)
 feed=root/'feed.xml';tree=module.ET.parse(feed);tree.find('channel/item/description').text='Record withdrawn; read the current project statement.';tree.write(feed,encoding='unicode')
 assert not module.check(root),module.check(root)
 # Canonical description and locale set drift cannot be published silently.
 p=root/'en/devlog/discovery-first/index.html';p.write_text(p.read_text().replace('A clearer role, an honest example, and a choice to keep reading.','Different summary.'))
 assert any('canonical summary' in e for e in module.check(root))
 p.unlink();assert any('locale sets differ' in e for e in module.check(root))
# Two legitimate same-date items must be checked per record, sorted by slug.
with tempfile.TemporaryDirectory() as tmp:
 root=Path(tmp)
 for file in files:
  target=root/file;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(module.ROOT/file,target)
 for prefix in ['', 'en/']:
  source=root/prefix/'devlog/discovery-first/index.html'
  target=root/prefix/'devlog/second-note/index.html';target.parent.mkdir(parents=True)
  target.write_text(source.read_text().replace('discovery-first','second-note'))
  for rel in ['index.html','devlog/index.html']:
   p=root/prefix/rel;s=p.read_text()
   match=re.search(r'<article[^>]*data-content-summary="discovery-first".*?</article>',s,re.S);assert match
   s=s[:match.end()]+match.group().replace('discovery-first','second-note')+s[match.end():];p.write_text(s)
  feed=root/prefix/'feed.xml';tree=module.ET.parse(feed);channel=tree.getroot().find('channel')
  import copy
  item=copy.deepcopy(channel.find('item'))
  for field in ['link','guid']:item.find(field).text=item.find(field).text.replace('discovery-first','second-note')
  channel.append(item);tree.write(feed,encoding='unicode')
 assert not module.check(root),module.check(root)
 p=root/'index.html';s=p.read_text();rows=re.findall(r'<article[^>]*data-content-summary="(?:discovery-first|second-note)".*?</article>',s,re.S);assert len(rows)==2
 p.write_text(s.replace(rows[0]+rows[1],rows[1]+rows[0]));assert any('published order' in e for e in module.check(root))
# Zero public entries is an honest valid state.
with tempfile.TemporaryDirectory() as tmp:
 root=Path(tmp)
 for file in files:
  target=root/file;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(module.ROOT/file,target)
 for prefix in ['', 'en/']:
  shutil.rmtree(root/prefix/'devlog/discovery-first')
  for rel in ['index.html','devlog/index.html']:
   p=root/prefix/rel;p.write_text(re.sub(r'<article[^>]*data-content-summary="discovery-first".*?</article>','',p.read_text(),flags=re.S))
  feed=root/prefix/'feed.xml';tree=module.ET.parse(feed);channel=tree.getroot().find('channel');channel.remove(channel.find('item'));tree.write(feed,encoding='unicode')
 assert not module.check(root),module.check(root)
print('site-discovery-check.test: OK (negative cases, zero/one/multiple and safe withdrawal)')
