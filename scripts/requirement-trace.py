#!/usr/bin/env python3
"""Read-only Trace ID queries; source text remains authoritative (no CI gate).

Python 3.9+, standard library only. See requirement-trace.README.md for scope,
GitHub snapshot format, cache semantics and interpretation limits.
"""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
from urllib.parse import unquote, urlsplit

SCHEMA_VERSION = 1
ID_BODY = r'O7-REQ-[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}'
ID_RE = re.compile(r'(?<![\w-])' + ID_BODY + r'(?![\w-])')
FULL_ID = re.compile(ID_BODY + r'\Z')
FIELD = re.compile(r'Trace (ID|refs)\s*[:：]\s*')
EXAMPLE = re.compile(r'示例|模板|样例|代码样本|\b(?:examples?|templates?|samples?)\b', re.I)
LINK = re.compile(r'\[[^\]]*\]\(\s*(<?[^\s)>]+>?)')
EXCLUDED = {'.git', 'node_modules', 'target', '.venv', 'venv', 'third_party', '__pycache__'}
TASK_URL = re.compile(r'https://github\.com/([^/]+/[^/]+)/(issues|pull)/(\d+)(?:#[^?]*)?\Z')


def utc_now():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def diagnostic(code, message, source=None, line=None, certainty='needs_review'):
    return dict(code=code, message=message, source=source, line=line, certainty=certainty)


def clean_markdown(text):
    """Remove comments, fenced/indented samples, preserving source line numbers."""
    text = re.sub(r'<!--.*?(?:-->|\Z)', lambda m: '\n' * m.group().count('\n'), text, flags=re.S)
    result, fence = [], None
    for line in text.splitlines():
        match = re.match(r'^ {0,3}(`{3,}|~{3,})', line)
        if fence:
            if match and match[1][0] == fence[0] and len(match[1]) >= len(fence) and not line[match.end():].strip():
                fence = None
            result.append('')
        elif match:
            fence = match[1]
            result.append('')
        elif line.startswith(('    ', '\t')):
            result.append('')
        else:
            result.append(line)
    return result


def slug(text):
    text = re.sub(r'<[^>]+>', '', text)
    text = re.sub(r'\[([^\]]+)\]\([^)]*\)', r'\1', text)
    text = text.replace('`', '').replace('*', '').replace('_', '_').strip().lower()
    return re.sub(r'[^\w\- ]', '', text).replace(' ', '-')


def parse_markdown(text, source):
    """Parse explicit fields and table columns, never bare ID mentions.

    Formal fields may use inline-code values. A whole inline-code field is an
    example, not a declaration. Context and links stay verbatim for human review.
    """
    lines = clean_markdown(text)
    records, diagnostics, anchors = [], [], set()
    headings, section, counts = [], {}, {}
    current_anchor, pending_anchor, table = None, None, None
    parts = Path(source.get('path', '')).parts
    ignored_path = any(part.lower() in ('templates', 'examples', 'fixtures') for part in parts[:-1]) or (
        bool(parts) and parts[-1].lower() in ('pull_request_template.md', 'issue_template.md'))
    # Compute section boundaries before assigning each record its source context.
    bounds = [i for i, line in enumerate(lines) if re.match(r'^ {0,3}#{1,6}\s', line)]
    bounds = [-1] + bounds + [len(lines)]
    for start, end in zip(bounds, bounds[1:]):
        context = '\n'.join(lines[max(start, 0):end]).strip()
        for number in range(start + 1, end):
            section[number] = context
    for i, line in enumerate(lines):
        explicit = re.search(r'<a\s+(?:name|id)=["\']([^"\']+)["\'][^>]*>', line, re.I)
        if explicit:
            anchors.add(explicit[1])
            pending_anchor = explicit[1]
            current_anchor = explicit[1]
        heading = re.match(r'^ {0,3}(#{1,6})\s+(.+?)\s*#*$', line)
        if heading:
            level, title = len(heading[1]), heading[2]
            while headings and headings[-1][0] >= level:
                headings.pop()
            headings.append((level, title))
            base = slug(title)
            n = counts.get(base, 0)
            counts[base] = n + 1
            auto = base + ('-' + str(n) if n else '')
            anchors.add(auto)
            current_anchor = pending_anchor or auto
            pending_anchor = None
            table = None
        excluded = ignored_path or any(
            EXAMPLE.search(title) and (level > 1 or re.fullmatch(r'(?:examples?|templates?|samples?|示例|模板|样例)(?:\s*[:：].*)?', title, re.I))
            for level, title in headings)
        if excluded or not line.strip() or line.lstrip().startswith('>'):
            if not line.strip():
                table = None
            continue
        cells = [c.strip() for c in line.strip().strip('|').split('|')] if '|' in line else None
        fields = []
        if cells and i + 1 < len(lines) and re.match(r'^\s*\|?\s*:?-{3,}', lines[i + 1]):
            table = {j: 'ID' if re.fullmatch(r'Trace ID\s*[:：]?', c.strip('`* ')) else 'refs'
                     for j, c in enumerate(cells)
                     if re.fullmatch(r'Trace (?:ID|refs)\s*[:：]?', c.strip('`* '))}
            continue
        if cells and all(re.fullmatch(r':?-+:?', c) for c in cells):
            continue
        if table and cells:
            fields.extend((role, cells[j], False) for j, role in table.items() if j < len(cells))
        # Explicit marker must begin a paragraph/list item or table cell.
        candidates = cells if cells else [re.sub(r'^\s*(?:[-*+]\s+|\d+[.)]\s+)?', '', line)]
        for candidate in candidates:
            candidate = re.sub(r'^`(Trace (?:ID|refs)\s*[:：])`\s*', r'\1 ', candidate)
            field = FIELD.match(candidate)
            if field:
                fields.append((field[1], candidate[field.end():], False))
            elif FIELD.search(candidate) and not re.search(r'`[^`]*Trace (?:ID|refs)', candidate):
                diagnostics.append(diagnostic('ambiguous_field', 'Field is not in a supported formal position; inspect original text.', source, i + 1))
        for role, payload, _ in fields:
            # Stop at prose after a formal field; only comma/semicolon/space-separated
            # IDs or Markdown ID links are accepted. A later successor is not a declaration.
            payload = re.sub(r'\[(' + ID_BODY + r')\]\([^)]*\)', r'\1', payload)
            payload = payload.strip()
            ids = []
            remainder = payload
            while remainder:
                match = re.match(r'`?(' + ID_BODY + r')`?(?![\w-])', remainder)
                if not match:
                    break
                ids.append(match[1])
                remainder = remainder[match.end():]
                separator = re.match(r'\s*[,;，；、]\s*|\s+(?=`?O7-REQ-)', remainder)
                if not separator:
                    break
                remainder = remainder[separator.end():]
            if not ids:
                if 'O7-REQ-' in payload:
                    diagnostics.append(diagnostic('invalid_id', 'Formal field has no valid complete lowercase UUID v4 ID.', source, i + 1))
                else:
                    diagnostics.append(diagnostic('ambiguous_field', 'Formal field has no concrete ID; inspect the source.', source, i + 1))
                continue
            if role == 'ID' and len(ids) != 1:
                diagnostics.append(diagnostic('ambiguous_declaration', 'A declaration field names multiple IDs; no unique ownership inferred.', source, i + 1))
                continue
            context = line.strip() if cells else section.get(i, line.strip())
            if re.search(r'拆分|合并|替代|退役|前身|后继|\b(?:split|merge|supersed\w*|retir\w*|predecessor|successor)\b', context, re.I):
                diagnostics.append(diagnostic('lifecycle_manual_review', 'Lifecycle wording preserved verbatim; inspect predecessor/successor links manually.', source, i + 1))
            links = list(dict.fromkeys(m[1].strip('<>') for m in LINK.finditer(context)))
            # Preserve reference-style Markdown links too; unresolved labels are explicit.
            definitions = dict(re.findall(r'^\s*\[([^\]]+)\]:\s*(\S+)', '\n'.join(lines), re.M))
            for match in re.finditer(r'\[[^\]]+\]\[([^\]]+)\]', context):
                if match[1] in definitions:
                    links.append(definitions[match[1]].strip('<>'))
                else:
                    diagnostics.append(diagnostic('unresolved_link_label', match[1], source, i + 1))
            for trace_id in dict.fromkeys(ids):
                record = dict(id=trace_id, role='declaration' if role == 'ID' else 'reference',
                              line=i + 1, anchor=current_anchor, context=context, links=links, source=dict(source))
                if record not in records:
                    records.append(record)
    return dict(records=records, anchors=sorted(anchors), diagnostics=diagnostics)


def git_value(root, *args):
    try:
        result = subprocess.run(['git', '-C', str(root), *args], capture_output=True, text=True, timeout=10)
        return result.stdout.strip() if result.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired):
        return None


def snapshot_freshness(snapshot, max_age=24):
    fetched = snapshot.get('fetched_at')
    try:
        stamp = dt.datetime.fromisoformat(fetched.replace('Z', '+00:00'))
        if stamp.tzinfo is None:
            raise ValueError('timezone required')
        age = (dt.datetime.now(dt.timezone.utc) - stamp).total_seconds()
        return dict(fetched_at=fetched, age_seconds=round(age, 3), status='fresh' if 0 <= age <= max_age * 3600 else 'stale', max_age_hours=max_age)
    except (AttributeError, TypeError, ValueError):
        return dict(fetched_at=fetched, age_seconds=None, status='unknown', max_age_hours=max_age)


def check_link(link, record, root, documents):
    """Check one locator in isolation, never following a local symlink.

    Walk lexical path components with lstat, including before '..', so a symlink
    cannot be hidden by normalization or redirect inspection outside the corpus.
    """
    check = dict(link=link, status='unverified_external')
    try:
        parsed = urlsplit(link)
    except (ValueError, TypeError):
        check['status'] = 'invalid_url'
        return check
    if parsed.scheme or parsed.netloc or record['source']['kind'] != 'document':
        return check
    try:
        target = ((root / record['source']['path']).parent / unquote(parsed.path)
                  if parsed.path else root / record['source']['path'])
        try:
            parts = target.relative_to(root).parts
        except ValueError:
            check['status'] = 'outside_scope'
            return check
        cursor = root
        for position, part in enumerate(parts):
            if part == '..':
                if cursor == root:
                    check['status'] = 'outside_scope'
                    return check
                cursor = cursor.parent
                continue
            cursor = cursor / part
            try:
                mode = cursor.lstat().st_mode
            except FileNotFoundError:
                check['status'] = 'path_not_found_in_working_tree'
                return check
            if stat.S_ISLNK(mode):
                check['status'] = 'unverifiable_symlink_not_followed'
                return check
            if position < len(parts) - 1 and not stat.S_ISDIR(mode):
                check['status'] = 'unverifiable_local_path'
                return check
        rel = cursor.relative_to(root).as_posix()
        check['target'] = rel
        if rel not in documents:
            check['status'] = 'unverified_outside_document_scope'
        elif parsed.fragment and unquote(parsed.fragment) not in documents[rel]:
            check['status'] = 'anchor_not_found_in_supported_markdown'
        else:
            check['status'] = 'resolved_in_working_tree'
    except (OSError, RuntimeError, ValueError):
        # A bad URL, path, permission, loop or filesystem error affects this link
        # only. Do not expose arbitrary exception text or discard formal records.
        check['status'] = 'unverifiable_local_path'
    return check


def build_index(root, github=None, repository=None, revision=None):
    root = Path(root).resolve()
    if not root.is_dir():
        raise ValueError('repository root must be an existing directory')
    commit = git_value(root, 'rev-parse', 'HEAD')
    local = dict(kind='repository', repository=repository, commit=commit,
                 requested_revision=revision, version_kind='working_tree',
                 dirty=bool(git_value(root, 'status', '--porcelain')) if commit else None,
                 scope='UTF-8 *.md files under root; excluded directories: ' + ', '.join(sorted(EXCLUDED)),
                 complete=True, errors=[], files=0, fetched_at=utc_now())
    records, diagnostics, documents = [], [], {}
    def walk_error(exc):
        local['complete'] = False
        local['errors'].append(dict(reason=type(exc).__name__, path=str(exc.filename)))
    for directory, dirs, files in os.walk(root, onerror=walk_error, followlinks=False):
        dirs[:] = sorted(d for d in dirs if d not in EXCLUDED and not Path(directory, d).is_symlink())
        for filename in sorted(files):
            path = Path(directory, filename)
            if path.suffix.lower() != '.md':
                continue
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                local['complete'] = False
                local['errors'].append(dict(path=relative, reason='symlink_not_followed'))
                continue
            try:
                content = path.read_text(encoding='utf-8')
            except (OSError, UnicodeError) as exc:
                local['complete'] = False
                local['errors'].append(dict(path=relative, reason=type(exc).__name__))
                continue
            source = dict(kind='document', path=relative, repository=repository, commit=commit,
                          version_kind='working_tree', sha256=hashlib.sha256(content.encode()).hexdigest())
            parsed = parse_markdown(content, source)
            documents[relative] = parsed['anchors']
            records.extend(parsed['records'])
            diagnostics.extend(parsed['diagnostics'])
            local['files'] += 1
    gh_source = dict(kind='github', status='unavailable', complete=False, scope='Issue and PR bodies only; no comments or Project fields', reason='offline_not_requested')
    if github is not None:
        if repository and github.get('repository') != repository:
            raise ValueError('GitHub snapshot repository does not match local repository identity')
        if not isinstance(github.get('coverage', {}), dict) or not isinstance(github.get('records'), list):
            raise ValueError('invalid GitHub snapshot coverage or records')
        gh_source.update(repository=github.get('repository'), fetched_at=github.get('fetched_at'),
                         coverage=github.get('coverage', {}), freshness=snapshot_freshness(github),
                         index_delay='REST listing avoids search index; pagination is not a transactional snapshot',
                         reason=github.get('refresh_error'), known_links=github.get('known_links', []))
        coverage = github.get('coverage', {})
        if any(not isinstance(coverage.get(k, {}), dict) for k in ('issues', 'pull_requests')):
            raise ValueError('snapshot coverage scopes must be objects')
        complete = snapshot_complete(github)
        gh_source['complete'] = complete and gh_source['freshness']['status'] == 'fresh'
        gh_source['status'] = ('complete_in_snapshot_scope' if complete else 'partial') if gh_source['freshness']['status'] == 'fresh' else 'stale_or_unknown_snapshot'
        for item in github.get('records', []):
            if (not isinstance(item, dict) or item.get('kind') not in ('issue', 'pull_request')
                    or not isinstance(item.get('body', ''), (str, type(None)))
                    or not isinstance(item.get('url'), str) or not isinstance(item.get('number'), int)):
                diagnostics.append(diagnostic('invalid_github_record', 'Unsupported task record; not interpreted as a formal source.'))
                gh_source['complete'] = False
                gh_source['status'] = 'partial'
                continue
            source = {k: item.get(k) for k in ('kind', 'number', 'url', 'state', 'updated_at', 'merged_at', 'head_sha', 'title')}
            source.update(repository=github.get('repository'), fetched_at=github.get('fetched_at'), version_kind='github_snapshot')
            parsed = parse_markdown(item.get('body') or '', source)
            records.extend(parsed['records'])
            diagnostics.extend(parsed['diagnostics'])
    by_id = {}
    for record in records:
        by_id.setdefault(record['id'], []).append(record)
        record['locator'] = (record['source'].get('url') or record['source'].get('path', '')) + ('#' + record['anchor'] if record['anchor'] else '')
        record['link_checks'] = []
        for link in record['links']:
            check = check_link(link, record, root, documents)
            if check['status'] in ('path_not_found_in_working_tree', 'anchor_not_found_in_supported_markdown'):
                diagnostics.append(diagnostic('broken_locator', check['status'] + ': ' + link, record['source'], record['line'], 'within_local_scope'))
            elif check['status'] in ('invalid_url', 'unverifiable_local_path', 'unverifiable_symlink_not_followed'):
                diagnostics.append(diagnostic('unverifiable_locator', check['status'] + ': ' + link, record['source'], record['line']))
            record['link_checks'].append(check)
    for trace_id, matches in sorted(by_id.items()):
        declarations = [r for r in matches if r['role'] == 'declaration']
        if len(declarations) > 1:
            diagnostics.append(dict(code='duplicate_declaration', id=trace_id, certainty='confirmed_in_observed_sources',
                                    message='Multiple formal declarations; references are not counted.', locations=[r['locator'] for r in declarations]))
        elif not declarations:
            diagnostics.append(dict(code='unresolved_reference', id=trace_id, certainty='needs_review',
                                    message='No declaration observed in scanned sources; not a global missing claim.', locations=[r['locator'] for r in matches]))
    return dict(schema_version=SCHEMA_VERSION, generated_at=utc_now(), records=records,
                diagnostics=diagnostics, sources=[local, gh_source], documents=documents,
                interpretation='Shared IDs show association only, never acceptance, publication or completion. Context includes applicability and lifecycle text; no semantic successor inference.')


def query_index(index, trace_id=None, document=None, task=None):
    if sum(value is not None for value in (trace_id, document, task)) != 1:
        raise ValueError('choose exactly one of trace_id, document or task')
    if trace_id is not None and not FULL_ID.fullmatch(trace_id):
        raise ValueError('expected a complete lowercase O7-REQ-UUID-v4')
    if trace_id:
        selected = [r for r in index['records'] if r['id'] == trace_id]
    else:
        selected = [r for r in index['records'] if (document is not None and r['source'].get('path') == document)
                    or (task is not None and r['source'].get('url') == task)]
    ids = sorted({r['id'] for r in selected})
    # Reverse query returns the whole associated chain plus exact originating hits.
    records = selected if trace_id else [r for r in index['records'] if r['id'] in ids]
    result = {k: v for k, v in index.items() if k not in ('records', 'documents')}
    groups = dict(declarations=[], design_references=[], tasks=[], other_document_references=[])
    for record in records:
        if record['role'] == 'declaration':
            key = 'declarations'
        elif record['source']['kind'] in ('issue', 'pull_request'):
            key = 'tasks'
        elif record['source'].get('path', '').endswith('.design.md'):
            key = 'design_references'
        else:
            key = 'other_document_references'
        groups[key].append(record)
    result.update(records=records, groups=groups, grouping_basis='Explicit declaration/ref role; design filename suffix; Issue/PR source kind. No inferred product applicability.', ids=ids, matched_records=selected,
                  query=dict(trace_id=trace_id, document=document, task=task),
                  outcome='found' if selected else 'not_found_in_observed_sources',
                  coverage='partial' if any(not s.get('complete') for s in index['sources']) else 'complete_only_in_declared_snapshot_scopes')
    return result


def gh_error(result):
    """Do not echo gh stderr: it may contain tokens, URLs or credential details."""
    stderr = (result.stderr or '').lower()
    for terms, reason in [(['429', 'rate limit'], 'rate_limited'), (['401', '403', 'permission', 'forbidden'], 'permission_denied'), (['404', 'not found'], 'not_found_or_not_authorized')]:
        if any(term in stderr for term in terms):
            return reason
    return 'gh_request_failed_exit_' + str(result.returncode)


def snapshot_complete(snapshot):
    coverage = snapshot.get('coverage', {})
    known = snapshot.get('known_links', [])
    if not isinstance(coverage, dict) or any(not isinstance(coverage.get(k, {}), dict) for k in ('issues', 'pull_requests')):
        raise ValueError('snapshot coverage scopes must be objects')
    if not isinstance(known, list) or any(not isinstance(item, dict) for item in known):
        raise ValueError('snapshot known_links must be objects in a list')
    return all(coverage.get(k, {}).get('complete') is True for k in ('issues', 'pull_requests')) and all(
        item.get('status') in ('read', 'outside_requested_repository') for item in known)


def fetch_github(repository, runner=None, known_urls=()):
    """Read REST pages via existing gh auth; no search cap, writes or new grants."""
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*', repository):
        raise ValueError('repository must be OWNER/REPO')
    runner = runner or subprocess.run
    snapshot = dict(schema_version=SCHEMA_VERSION, repository=repository, fetched_at=utc_now(), records=[], coverage={})
    items = {}
    def request(endpoint):
        try:
            result = runner(['gh', 'api', '--method', 'GET', endpoint], text=True, capture_output=True, timeout=30)
        except subprocess.TimeoutExpired:
            return None, 'timeout'
        except OSError:
            return None, 'gh_unavailable'
        if result.returncode:
            return None, gh_error(result)
        try:
            return json.loads(result.stdout), None
        except (ValueError, TypeError):
            return None, 'invalid_json_response'
    def retain(value, kind):
        if not isinstance(value, dict) or not isinstance(value.get('number'), int) or not isinstance(value.get('html_url', value.get('url')), str):
            return False
        if not isinstance(value.get('body'), (str, type(None))) or not isinstance(value.get('head') or {}, dict):
            return False
        if kind == 'issue' and value.get('pull_request'):
            return True
        item = dict(kind=kind, number=value['number'], url=value.get('html_url', value.get('url')),
                    body=value.get('body') or '', title=value.get('title'), state=value.get('state'),
                    updated_at=value.get('updated_at'), merged_at=value.get('merged_at'),
                    head_sha=(value.get('head') or {}).get('sha'))
        items[(kind, item['number'])] = item
        return True
    for name, endpoint, kind in [('issues', 'issues', 'issue'), ('pull_requests', 'pulls', 'pull_request')]:
        scope = dict(complete=False, pages=0, state_filter='all', endpoint='repos/' + repository + '/' + endpoint,
                     permissions='existing gh identity; visible records only, inaccessible records unknowable', errors=[])
        snapshot['coverage'][name] = scope
        page, seen = 1, set()
        while True:
            data, error = request(scope['endpoint'] + '?state=all&per_page=100&page=' + str(page))
            if error or not isinstance(data, list):
                scope['errors'].append(error or 'unexpected_response_shape')
                break
            fingerprint = hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()
            if data and fingerprint in seen:
                scope['errors'].append('repeated_page')
                break
            seen.add(fingerprint)
            scope['pages'] += 1
            valid = [retain(value, kind) for value in data]
            if not all(valid):
                scope['errors'].append('invalid_record')
                break
            if len(data) < 100:
                scope['complete'] = True
                break
            page += 1
    direct = []
    for url in sorted(set(known_urls)):
        match = TASK_URL.fullmatch(url)
        if not match or match[1] != repository:
            direct.append(dict(url=url, status='outside_requested_repository'))
            continue
        kind = 'issue' if match[2] == 'issues' else 'pull_request'
        endpoint = 'issues' if kind == 'issue' else 'pulls'
        data, error = request('repos/' + repository + '/' + endpoint + '/' + match[3])
        ok = not error and retain(data, kind)
        direct.append(dict(url=url, status='read' if ok else (error or 'invalid_record')))
    snapshot['known_links'] = direct
    snapshot['records'] = sorted(items.values(), key=lambda x: (x['kind'], x['number']))
    snapshot['completed_at'] = utc_now()
    return snapshot


def load_snapshot(path):
    value = json.loads(Path(path).read_text(encoding='utf-8'))
    if not isinstance(value, dict) or not isinstance(value.get('records'), list):
        raise ValueError('snapshot must be an object containing records')
    return value


def validate_cache_path(path, root):
    """Only a dedicated outside-root JSON cache is eligible for writes."""
    path, root = Path(path).absolute(), Path(root).resolve()
    try:
        path.resolve().relative_to(root)
    except ValueError:
        pass
    else:
        raise ValueError('--cache must be outside repository root; source files are never overwritten')
    if path.is_symlink():
        raise ValueError('refusing a symlink cache')
    if path.suffix.lower() != '.json':
        raise ValueError('--cache must use a dedicated .json file')
    return path


def write_cache(path, snapshot, root):
    """Atomically replace only an explicitly selected derived cache."""
    path = validate_cache_path(path, root)
    if path.exists():
        previous = load_snapshot(path)
        if not isinstance(previous.get('coverage'), dict) or not previous.get('repository') or not previous.get('fetched_at'):
            raise ValueError('existing cache is not a recognized GitHub trace snapshot')
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix='.trace-cache-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as stream:
            json.dump(snapshot, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


def render_text(result):
    lines = ['Trace query: ' + result.get('outcome', 'rebuilt derived index'), result['interpretation']]
    for source in result['sources']:
        lines.append('Source: ' + json.dumps(source, ensure_ascii=False, sort_keys=True))
    groups = result.get('groups', {'observed_records': result['records']})
    for name, records in groups.items():
        lines.append('\n' + name + ' (' + str(len(records)) + ')')
        for record in records:
            lines.append(record['id'] + ' ' + record['role'] + ' ' + record['locator'] + ':' + str(record['line']))
            lines.append(record['context'])
            for check in record.get('link_checks', []):
                lines.append('  Link: ' + check['link'] + ' [' + check['status'] + ']')
    for item in result['diagnostics']:
        lines.append('Diagnostic: ' + json.dumps(item, ensure_ascii=False, sort_keys=True))
    return '\n'.join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', default='.', help='local checkout or exported Markdown corpus (read-only)')
    parser.add_argument('--repository', help='source repository identity OWNER/REPO')
    parser.add_argument('--revision', help='claimed source revision, kept separately from detected checkout HEAD')
    parser.add_argument('--format', choices=['json', 'text'], default='text')
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--github-snapshot', help='offline JSON snapshot; does not contact GitHub')
    group.add_argument('--github', metavar='OWNER/REPO', help='opt in to read-only gh REST listing, all states and pages')
    parser.add_argument('--known-url', action='append', default=[], help='read this known issue/PR URL directly with --github')
    parser.add_argument('--cache', help='explicit derived GitHub cache outside repository; optional')
    parser.add_argument('--refresh', action='store_true', help='refresh --github cache; keep old records on failure')
    parser.add_argument('--max-age', type=float, default=24, help='cache freshness in hours (default 24)')
    sub = parser.add_subparsers(dest='command', required=True)
    query = sub.add_parser('query', help='exact complete ID query')
    query.add_argument('id')
    reverse = sub.add_parser('reverse', help='document or task to associated IDs and source chains')
    selector = reverse.add_mutually_exclusive_group(required=True)
    selector.add_argument('--document', help='exact root-relative Markdown path')
    selector.add_argument('--task', help='exact GitHub task URL')
    sub.add_parser('rebuild', help='rebuild the derived index to stdout; never edits sources')
    args = parser.parse_args(argv)
    if args.refresh and not args.github:
        parser.error('--refresh requires --github')
    if args.known_url and not args.github:
        parser.error('--known-url requires --github')
    if args.github_snapshot and args.cache:
        parser.error('choose --github-snapshot or --cache')
    if not math.isfinite(args.max_age) or args.max_age < 0:
        parser.error('--max-age must be finite and nonnegative')
    try:
        if args.command == 'query' and not FULL_ID.fullmatch(args.id):
            raise ValueError('expected a complete lowercase O7-REQ-UUID-v4')
        if args.cache:
            validate_cache_path(args.cache, args.root)
        old = load_snapshot(args.cache) if args.cache and Path(args.cache).exists() else None
        if old and args.github and old.get('repository') != args.github:
            raise ValueError('cache repository does not match --github')
        github = load_snapshot(args.github_snapshot) if args.github_snapshot else old
        if args.github and (github is None or args.refresh or args.known_url or (args.command == 'reverse' and args.task)):
            known = list(args.known_url)
            if args.command == 'reverse' and args.task:
                known.append(args.task)
            # Follow existing task links in local formal context, same repository only.
            local = build_index(args.root, repository=args.repository or args.github, revision=args.revision)
            known.extend(link for r in local['records'] for link in r['links'] if TASK_URL.fullmatch(link))
            fresh = fetch_github(args.github, known_urls=known)
            if old and not snapshot_complete(fresh):
                github = dict(old)
                github['refresh_error'] = dict(at=fresh['fetched_at'], coverage=fresh['coverage'], known_links=fresh.get('known_links', []), reason='refresh_incomplete_previous_snapshot_preserved')
            else:
                github = fresh
                if args.cache:
                    write_cache(args.cache, github, args.root)
        index = build_index(args.root, github, args.repository or args.github or (github or {}).get('repository'), args.revision)
        if github:
            gh_source = index['sources'][1]
            gh_source['freshness'] = snapshot_freshness(github, args.max_age)
            if gh_source['freshness']['status'] != 'fresh':
                gh_source['complete'] = False
                gh_source['status'] = 'stale_or_unknown_snapshot'
            elif gh_source['status'] == 'stale_or_unknown_snapshot':
                gh_source['complete'] = snapshot_complete(github)
                gh_source['status'] = 'complete_in_snapshot_scope' if gh_source['complete'] else 'partial'
            if github.get('refresh_error'):
                index['sources'][1]['complete'] = False
                index['sources'][1]['status'] = 'partial_previous_snapshot'
        result = index if args.command == 'rebuild' else query_index(index, trace_id=args.id if args.command == 'query' else None,
                    document=args.document if args.command == 'reverse' else None, task=args.task if args.command == 'reverse' else None)
        print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) if args.format == 'json' else render_text(result))
        return 0
    except (OSError, ValueError, TypeError, KeyError) as exc:
        # No credential values or arbitrary subprocess stderr are included.
        print(json.dumps(dict(error='input_or_source_error', message=(type(exc).__name__ if isinstance(exc, OSError) else str(exc))), ensure_ascii=False), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
