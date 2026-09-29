#!/usr/bin/env python3
"""Supplemental admission regressions for closed-duplicate candidate aliases.

The frozen retirement acceptance suite covers the explicit retirement path.
These tests protect the earlier admission edge: a stale candidate cache row
must not be bootstrapped as an active task before that retirement is run.
"""
from __future__ import annotations

import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PM = ROOT / "scripts/pm"
BOOTSTRAP = PM / "bootstrap-task-snapshot.py"
WORKFLOW_NEXT = PM / "workflow-next.py"
PROJECT_SYNC = PM / "github-project-sync.py"
FIXTURE_TEST = PM / "retire-closed-duplicate-candidate.test.py"


def load_fixture_module():
    spec = importlib.util.spec_from_file_location("retirement_fixture_for_bootstrap_guard", FIXTURE_TEST)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load isolated retirement fixture from {FIXTURE_TEST}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


FIXTURE = load_fixture_module()


class ClosedDuplicateBootstrapGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="closed-duplicate-bootstrap-guard-")
        self.fixture = FIXTURE.RetirementFixture(Path(self.temp.name))
        self._prepare_active_looking_candidate()

    def tearDown(self):
        self.temp.cleanup()

    def _prepare_active_looking_candidate(self):
        """Make the cached candidate look sufficient for ordinary bootstrap."""
        subprocess.run(
            ["git", "-C", str(self.fixture.foreign_worktree), "checkout", FIXTURE.FOREIGN_BRANCH],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        for uid in (FIXTURE.CANDIDATE_UID, FIXTURE.FOREIGN_UID):
            record = mapping["tasks"][uid]
            record.update({
                "title": "fixture task title",
                "acceptance": ["snapshot must remain absent on rejection"],
                "default_branch": "main",
            })
        self.fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _remove_foreign_owner_mapping(self):
        """Leave the candidate's cached path/branch unowned so alias-only guards miss it."""
        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        del mapping["tasks"][FIXTURE.FOREIGN_UID]
        self.fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _write_unavailable_gh_stub(self):
        self.fixture.bin.mkdir(parents=True, exist_ok=True)
        (self.fixture.bin / "gh").write_text(
            "#!/bin/sh\n"
            "echo 'fixture: live GitHub authority unavailable' >&2\n"
            "exit 73\n",
            encoding="utf-8",
        )
        (self.fixture.bin / "gh").chmod(0o755)

    def _run_bootstrap(self, uid=FIXTURE.CANDIDATE_UID):
        return subprocess.run(
            [
                sys.executable,
                str(BOOTSTRAP),
                "validate-or-create",
                "--repo-root", str(self.fixture.foreign_worktree),
                "--tasks-json", str(self.fixture.mapping),
                "--task-uid", uid,
                "--producer", "closed-duplicate-bootstrap-guard-test",
            ],
            cwd=ROOT,
            env=self.fixture.environment(),
            text=True,
            capture_output=True,
            check=False,
            timeout=15,
        )

    def _run_workflow_next(self):
        return subprocess.run(
            [
                sys.executable,
                str(WORKFLOW_NEXT),
                "--repo-root", str(self.fixture.foreign_worktree),
                "--mapping", str(self.fixture.mapping),
                "--task-uid", FIXTURE.CANDIDATE_UID,
                "--json",
            ],
            cwd=ROOT,
            env=self.fixture.environment(),
            text=True,
            capture_output=True,
            check=False,
            timeout=15,
        )

    def _snapshot_path(self):
        return self.fixture.foreign_worktree / ".pm/scratch" / FIXTURE.CANDIDATE_UID / "bootstrap-task-snapshot.json"

    def _write_sync_task(self, task_uid=FIXTURE.CANDIDATE_UID):
        source_tasks = self.fixture.mapping_root / ".pm/tasks"
        source_tasks.mkdir(parents=True, exist_ok=True)
        task_path = source_tasks / f"{task_uid}.yaml"
        task_path.write_text(
            "\n".join([
                f"task_uid: {task_uid}",
                "title: stale duplicate candidate" if task_uid == FIXTURE.CANDIDATE_UID else "valid new task",
                "owner_role: repository_health_engineer",
                "module: engineering",
                'worktree_hint: ""',
                "workflow_phase: bootstrap",
                f"execution_log_path: .pm/tasks/{task_uid}.execution.md",
                "status: candidate",
                "priority: P2",
                "source_refs: []",
                "doc_refs: []",
                "related_prd: []",
                "acceptance: []",
                "handoff_to: []",
                "updated_at: 2026-09-27T00:00:00Z",
                "",
            ]),
            encoding="utf-8",
        )
        return task_path

    def _remove_candidate_mapping_row(self):
        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        del mapping["tasks"][FIXTURE.CANDIDATE_UID]
        self.fixture.mapping.write_text(json.dumps(mapping, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def _write_global_sync_gh_stub(self):
        """Expose the exact duplicate Issue/Project fixture to the real sync CLI."""
        proof = self.fixture.live_proof()
        candidate_issue = next(
            issue for issue in proof["issues"]["items"] if issue["number"] == FIXTURE.CANDIDATE_ISSUE
        )
        candidate_item = next(
            item for item in proof["project_items"]["items"] if item["issue_number"] == FIXTURE.CANDIDATE_ISSUE
        )
        if candidate_issue["state"] != "CLOSED" or candidate_issue["state_reason"] != "DUPLICATE":
            raise AssertionError("sync regression requires the canonical closed-DUPLICATE Issue fixture")
        if candidate_issue["body"].splitlines().count(f"task_uid: {FIXTURE.CANDIDATE_UID}") != 1:
            raise AssertionError("sync regression Issue must have one exact canonical Task UID")
        if (
            candidate_item["task_uid"] != FIXTURE.CANDIDATE_UID
            or candidate_item["issue_number"] != FIXTURE.CANDIDATE_ISSUE
            or candidate_item["project_owner"] != "eng-cc"
            or candidate_item["project_number"] != 1
            or candidate_item["project_id"] != "PVT_fixture"
            or candidate_item["archived"] is not False
        ):
            raise AssertionError("sync regression Project item must bind the exact canonical Issue and Project")
        if candidate_item["fields"] != {
            "Status": "Done",
            "PM Status": "candidate",
            "Workflow Phase": "bootstrap",
            "Canonical Worktree": "",
        }:
            raise AssertionError("sync regression requires the exact canonical candidate Project fields")
        sync_fixture = self.fixture.directory / "global-sync-github-fixture.json"
        sync_fixture.write_text(
            json.dumps({
                "issues": proof["issues"]["items"],
                "project_items": proof["project_items"]["items"],
                "comments": proof["candidate_comments"]["items"],
                "permissions": proof["repository_permissions"]["items"],
                "pull_requests": proof["artifact_discovery"]["pull_requests"],
                "project": proof["project"],
            }, sort_keys=True),
            encoding="utf-8",
        )
        gh = self.fixture.bin / "gh"
        gh.write_text(
            textwrap.dedent("""\
                #!/usr/bin/env python3
                import json, os, pathlib, re, sys
                args = sys.argv[1:]
                fixture = json.loads(pathlib.Path(os.environ['SYNC_GH_FIXTURE']).read_text(encoding='utf-8'))

                def log(path, value):
                    with open(path, 'a', encoding='utf-8') as handle:
                        handle.write(json.dumps(value, sort_keys=True) + '\\n')

                log(os.environ['SYNC_GH_LOG'], {'args': args})

                def selected(query, name):
                    return re.search(r'\\b' + re.escape(name) + r'\\b', query) is not None

                def selection_body(source, field):
                    match = re.search(
                        r'(?:\\b[A-Za-z_][A-Za-z_0-9]*\\s*:\\s*)?\\b' + re.escape(field)
                        + r'\\s*(?:\\([^{}]*\\))?\\s*\\{', source
                    )
                    if not match:
                        return ''
                    start = match.end()
                    depth = 1
                    for index in range(start, len(source)):
                        if source[index] == '{':
                            depth += 1
                        elif source[index] == '}':
                            depth -= 1
                            if depth == 0:
                                return source[start:index]
                    return ''

                def direct_selection(source):
                    direct = []
                    depth = 0
                    for char in source:
                        if char == '{':
                            depth += 1
                        elif char == '}':
                            depth -= 1
                        elif depth == 0:
                            direct.append(char)
                    return ''.join(direct)

                def selected_direct(source, name):
                    return re.search(r'\\b' + re.escape(name) + r'\\b', direct_selection(source)) is not None

                def exact_uid(issue, uid):
                    return str(issue.get('body') or '').splitlines().count('task_uid: ' + uid) == 1

                def issue_for_item(item):
                    return next(issue for issue in fixture['issues'] if issue['number'] == item['issue_number'])

                def issue_node(issue, query, *, include_project_items=True):
                    node = {'__typename': 'Issue'}
                    for field in ('number', 'url', 'body', 'state', 'stateReason'):
                        if selected(query, field):
                            source = {'url': 'url', 'stateReason': 'state_reason'}.get(field, field)
                            if source in issue:
                                node[field] = issue[source]
                    if include_project_items and re.search(r'\\bprojectItems\\s*\\(', query):
                        item_nodes = [project_item_node(item, query, include_content=False)
                                      for item in fixture['project_items']
                                      if item['issue_number'] == issue['number']]
                        connection = {'nodes': item_nodes}
                        project_items_query = selection_body(query, 'projectItems')
                        if selection_body(project_items_query, 'pageInfo'):
                            connection['pageInfo'] = {'hasNextPage': False, 'endCursor': None}
                        node['projectItems'] = connection
                    return node

                def project_item_node(item, query, *, include_content=True):
                    node = {'id': item['id']}
                    if selected(query, 'isArchived'):
                        node['isArchived'] = item['archived']
                    item_project_query = selection_body(query, 'project')
                    if item_project_query:
                        project = {}
                        if selected_direct(item_project_query, 'id'):
                            project['id'] = item['project_id']
                        if selected_direct(item_project_query, 'number'):
                            project['number'] = item['project_number']
                        owner_query = selection_body(item_project_query, 'owner')
                        if owner_query:
                            project['owner'] = {'__typename': 'Organization'}
                            if re.search(r'\\.\\.\\.\\s+on\\s+Organization\\s*\\{[^{}]*\\blogin\\b', owner_query):
                                project['owner']['login'] = item['project_owner']
                        node['project'] = project
                    if include_content and re.search(r'\\bcontent\\s*\\{', query):
                        node['content'] = issue_node(issue_for_item(item), query, include_project_items=False)
                    field_values_query = selection_body(query, 'fieldValues')
                    if field_values_query:
                        values = []
                        field_name_query = selection_body(field_values_query, 'field')
                        single_select_query = selection_body(field_values_query, 'ProjectV2ItemFieldSingleSelectValue')
                        text_value_query = selection_body(field_values_query, 'ProjectV2ItemFieldTextValue')
                        selects_field_name = selected(field_name_query, 'name')
                        for name, value in item['fields'].items():
                            field_value = {}
                            if selects_field_name:
                                field_value['field'] = {'name': name}
                            if name in ('Status', 'PM Status', 'Workflow Phase') and selected_direct(single_select_query, 'name'):
                                field_value['name'] = value
                            elif name == 'Canonical Worktree' and selected_direct(text_value_query, 'text'):
                                field_value['text'] = value
                            values.append(field_value)
                        connection = {'nodes': values}
                        if selection_body(field_values_query, 'pageInfo'):
                            connection['pageInfo'] = {'hasNextPage': False, 'endCursor': None}
                        node['fieldValues'] = connection
                    return node

                def issue_connection(query, issues):
                    nodes = [issue_node(issue, query) for issue in issues]
                    connection = {'nodes': nodes}
                    issue_query = selection_body(query, 'issues') or selection_body(query, 'search')
                    if selection_body(issue_query, 'pageInfo'):
                        connection['pageInfo'] = {'hasNextPage': False, 'endCursor': None}
                    return connection

                def project_connection(query):
                    nodes = [project_item_node(item, query) for item in fixture['project_items']]
                    connection = {'nodes': nodes}
                    project_query = selection_body(query, 'projectV2')
                    items_query = selection_body(project_query, 'items')
                    if selection_body(items_query, 'pageInfo'):
                        connection['pageInfo'] = {'hasNextPage': False, 'endCursor': None}
                    return connection

                def deliver(payload):
                    output = [payload] if '--paginate' in args and '--slurp' in args else payload
                    log(os.environ['SYNC_GH_READ_LOG'], {'args': args, 'payload': output})
                    print(json.dumps(output, sort_keys=True))

                def issue_rest(issue):
                    return {
                        'number': issue['number'], 'url': issue['url'], 'html_url': issue['url'],
                        'body': issue['body'], 'state': str(issue['state']).lower(),
                        'state_reason': str(issue.get('state_reason') or '').lower() or None,
                    }

                if args[:2] == ['api', 'graphql']:
                    query = next((value.split('=', 1)[1] for value in args if value.startswith('query=')), '')
                    joined = ' '.join(args)
                    if 'rateLimit' in query:
                        deliver({'data': {'rateLimit': {'remaining': 5000, 'resetAt': '2099-01-01T00:00:00Z'}}})
                    elif re.search(r'\\bsearch\\s*\\(', query):
                        requested_uid = next((uid for uid in (issue.get('body', '').split('task_uid: ')[-1].splitlines()[0]
                                                               for issue in fixture['issues'])
                                              if uid in joined), '')
                        matches = [issue for issue in fixture['issues'] if requested_uid and exact_uid(issue, requested_uid)]
                        alias_match = re.search(r'\\b([A-Za-z_][A-Za-z_0-9]*)\\s*:\\s*search\\s*\\(', query)
                        alias = alias_match.group(1) if alias_match else 'search'
                        data = {alias: issue_connection(query, matches)}
                        deliver({'data': data})
                    elif re.search(r'\\bissues\\s*\\(', query):
                        repository = {'issues': issue_connection(query, fixture['issues'])}
                        deliver({'data': {'repository': repository}})
                    elif re.search(r'\\bprojectV2\\s*\\(', query):
                        owner_root = 'organization' if 'organization(login:' in query else 'user'
                        project_query = selection_body(query, 'projectV2')
                        project = {}
                        if selected_direct(project_query, 'id'):
                            project['id'] = fixture['project']['id']
                        if selected_direct(project_query, 'number'):
                            project['number'] = fixture['project']['number']
                        owner_query = selection_body(project_query, 'owner')
                        if owner_query:
                            project['owner'] = {'__typename': 'Organization'}
                            if re.search(r'\\.\\.\\.\\s+on\\s+Organization\\s*\\{[^{}]*\\blogin\\b', owner_query):
                                project['owner']['login'] = fixture['project']['owner']
                        project['items'] = project_connection(query)
                        deliver({'data': {owner_root: {'projectV2': project}}})
                    elif re.search(r'\\bissue\\s*\\(', query):
                        selected_issue = fixture['issues'][0]
                        deliver({'data': {'repository': {'issue': issue_node(selected_issue, query)}}})
                    elif 'mutation' in query:
                        deliver({'data': {}})
                    else:
                        deliver({'data': {}})
                elif args[:1] == ['api']:
                    route = next((value for value in args[1:] if not value.startswith('-')), '')
                    match = re.search(r'/issues/(\\d+)(?:/|$|[?])', route)
                    issue_number = int(match.group(1)) if match else None
                    if issue_number and route.split('?')[0].endswith('/comments'):
                        deliver(fixture['comments'])
                    elif issue_number:
                        selected_issue = next(issue for issue in fixture['issues'] if issue['number'] == issue_number)
                        deliver(issue_rest(selected_issue))
                    elif route.split('?', 1)[0].endswith('/issues'):
                        deliver([issue_rest(issue) for issue in fixture['issues']])
                    elif '/pulls' in route:
                        deliver(fixture['pull_requests'])
                    elif '/collaborators/' in route:
                        deliver(fixture['permissions'][0])
                    else:
                        print('unsupported GH API fixture route ' + route, file=sys.stderr)
                        sys.exit(9)
                elif args[:2] == ['project', 'view']:
                    print(json.dumps({'id': fixture['project']['id'], 'number': fixture['project']['number']}))
                elif args[:2] == ['project', 'field-list']:
                    options = lambda values: [{'id': 'OPT_' + str(index), 'name': value} for index, value in enumerate(values)]
                    print(json.dumps({'fields': [
                        {'id':'FIELD_STATUS','name':'Status','type':'ProjectV2SingleSelectField','options':options(['Todo','In Progress','Blocked','Ready / PR','PR Watch','Done'])},
                        {'id':'FIELD_TASK_UID','name':'Task UID','type':'ProjectV2Field'},
                        {'id':'FIELD_OWNER','name':'Owner Role','type':'ProjectV2SingleSelectField','options':options(['repository_health_engineer'])},
                        {'id':'FIELD_MODULE','name':'Module','type':'ProjectV2SingleSelectField','options':options(['engineering'])},
                        {'id':'FIELD_PM_STATUS','name':'PM Status','type':'ProjectV2SingleSelectField','options':options(['candidate'])},
                        {'id':'FIELD_PHASE','name':'Workflow Phase','type':'ProjectV2SingleSelectField','options':options(['bootstrap'])},
                        {'id':'FIELD_PRIORITY','name':'Priority','type':'ProjectV2SingleSelectField','options':options(['P2'])},
                        {'id':'FIELD_WORKTREE','name':'Canonical Worktree','type':'ProjectV2Field'},
                        {'id':'FIELD_PR','name':'PR','type':'ProjectV2Field'},
                        {'id':'FIELD_TIER','name':'Test Tier Required','type':'ProjectV2SingleSelectField','options':options(['n/a'])}
                    ]}))
                elif args[:2] == ['project', 'item-list']:
                    print(json.dumps({'items': fixture['project_items']}))
                elif args[:2] == ['issue', 'create']:
                    print('https://github.com/eng-cc/oasis7/issues/900099')
                elif args[:2] == ['project', 'item-add']:
                    print(json.dumps({'id':'PVTI_created'}))
                elif args[:2] == ['project', 'item-edit']:
                    print('{}')
                else:
                    print('unexpected sync fixture command ' + json.dumps(args), file=sys.stderr)
                    sys.exit(9)
                """),
            encoding="utf-8",
        )
        gh.chmod(0o755)
        return sync_fixture

    def _run_global_sync(self, *, skip_recover):
        sync_fixture = self._write_global_sync_gh_stub()
        call_log = self.fixture.directory / "global-sync-gh-calls.jsonl"
        read_log = self.fixture.directory / "global-sync-gh-reads.jsonl"
        read_log.write_text("", encoding="utf-8")
        environment = self.fixture.environment()
        environment["SYNC_GH_FIXTURE"] = str(sync_fixture)
        environment["SYNC_GH_LOG"] = str(call_log)
        environment["SYNC_GH_READ_LOG"] = str(read_log)
        command = [
            sys.executable,
            str(PROJECT_SYNC),
            str(self.fixture.mapping_root),
            "--repo", "eng-cc/oasis7",
            "--project-owner", "eng-cc",
            "--project-number", "1",
            "--mapping", str(self.fixture.mapping),
            "--status", "candidate",
            "--global-maintenance",
            "--apply",
            "--json",
        ]
        if skip_recover:
            command.append("--skip-recover")
        result = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
            timeout=20,
        )
        calls = [json.loads(line) for line in call_log.read_text(encoding="utf-8").splitlines()]
        reads = [json.loads(line) for line in read_log.read_text(encoding="utf-8").splitlines()]
        return result, calls, reads

    @staticmethod
    def _walk_dicts(value):
        if isinstance(value, dict):
            yield value
            for child in value.values():
                yield from ClosedDuplicateBootstrapGuardTests._walk_dicts(child)
        elif isinstance(value, list):
            for child in value:
                yield from ClosedDuplicateBootstrapGuardTests._walk_dicts(child)

    @staticmethod
    def _issue_uid(issue):
        body = str(issue.get("body") or "")
        return body.splitlines().count(f"task_uid: {FIXTURE.CANDIDATE_UID}") == 1

    @staticmethod
    def _query_for_read(read):
        args = read.get("args") or []
        return next((argument.split("=", 1)[1] for argument in args if argument.startswith("query=")), "")

    @staticmethod
    def _graphql_payloads(payload):
        if isinstance(payload, dict):
            return [payload]
        if isinstance(payload, list) and payload and all(isinstance(page, dict) for page in payload):
            return payload
        return []

    @classmethod
    def _graphql_connection(cls, payload, query, kind):
        connections = []
        for page in cls._graphql_payloads(payload):
            data = page.get("data")
            if not isinstance(data, dict):
                continue
            if kind == "issue_search":
                alias_match = re.search(r"\b([A-Za-z_][A-Za-z_0-9]*)\s*:\s*search\s*\(", query)
                alias = alias_match.group(1) if alias_match else "search"
                connection = data.get(alias)
            elif kind == "issue_inventory":
                connection = (data.get("repository") or {}).get("issues")
            elif kind == "project_items":
                owner_root = data.get("organization") or data.get("user") or {}
                project = owner_root.get("projectV2") or data.get("projectV2") or {}
                connection = project.get("items")
            else:
                connection = None
            if isinstance(connection, dict):
                connections.append(connection)
        if not connections:
            return None
        nodes = [node for connection in connections for node in connection.get("nodes", [])]
        return {"nodes": nodes, "pageInfo": connections[-1].get("pageInfo")}

    @staticmethod
    def _rest_pages_complete(read):
        args = read.get("args") or []
        payload = read.get("payload")
        route = next((arg for arg in args[1:] if not arg.startswith("-")), "")
        if "--paginate" not in args or "--slurp" not in args or not route.split("?", 1)[0].endswith("/issues"):
            return False
        return isinstance(payload, list) and bool(payload) and all(isinstance(page, list) for page in payload)

    @staticmethod
    def _item_fields(item):
        fields = item.get("fields")
        if isinstance(fields, dict):
            return fields
        values = ((item.get("fieldValues") or {}).get("nodes") or [])
        result = {}
        for value in values:
            field_name = str(((value.get("field") or {}).get("name") or ""))
            if field_name:
                result[field_name] = str(value.get("name") or value.get("text") or "")
        return result

    @classmethod
    def _candidate_items(cls, payload):
        found = []

        def visit(value, candidate_issue=None, project_context=None):
            if isinstance(value, dict):
                context = candidate_issue
                if cls._issue_uid(value):
                    context = {
                        "number": value.get("number"),
                        "url": value.get("url") or value.get("html_url"),
                    }
                content = value.get("content")
                content_matches = isinstance(content, dict) and cls._issue_uid(content)
                project = value.get("project")
                if (
                    "items" in value
                    and value.get("id")
                    and value.get("number") is not None
                    and isinstance(value.get("owner"), dict)
                ):
                    project_context = {
                        "id": value.get("id"),
                        "number": value.get("number"),
                        "owner": value.get("owner"),
                    }
                if (
                    value.get("id")
                    and (isinstance(project, dict) or project_context is not None)
                    and ("fieldValues" in value or "content" in value or isinstance(project, dict))
                    and (context is not None or content_matches)
                ):
                    matched_issue = context or {
                        "number": content.get("number"),
                        "url": content.get("url") or content.get("html_url"),
                    }
                    found.append({"item": value, "issue": matched_issue, "project": project or project_context})
                for child in value.values():
                    visit(child, context, project_context)
            elif isinstance(value, list):
                for child in value:
                    visit(child, candidate_issue, project_context)

        visit(payload)
        return found

    @classmethod
    def _project_connection_complete(cls, payload, query):
        if re.search(r"\bprojectV2\s*\(", query):
            connection = cls._graphql_connection(payload, query, "project_items")
            info = connection.get("pageInfo") if isinstance(connection, dict) else None
            return isinstance(info, dict) and info.get("hasNextPage") is False
        if re.search(r"\bprojectItems\s*\(", query):
            return any(
                isinstance(value.get("projectItems"), dict)
                and isinstance(value["projectItems"].get("pageInfo"), dict)
                and value["projectItems"]["pageInfo"].get("hasNextPage") is False
                for value in cls._walk_dicts(payload)
            )
        return False

    @classmethod
    def _live_duplicate_violations(cls, reads):
        issue_reads = []
        project_reads = []
        all_issue_ids = set()
        all_project_ids = set()
        exact_issue_observed = False
        exact_project_observed = False

        for read in reads:
            args = read.get("args") or []
            request = " ".join(args)
            query = cls._query_for_read(read)
            payload = read.get("payload")
            if "rateLimit" in query:
                continue

            issue_kind = None
            if re.search(r"\bsearch\s*\(", query):
                issue_kind = "issue_search"
            elif re.search(r"\bissues\s*\(", query):
                issue_kind = "issue_inventory"
            if issue_kind:
                connection = cls._graphql_connection(payload, query, issue_kind)
                page_info = connection.get("pageInfo") if isinstance(connection, dict) else None
                complete = isinstance(page_info, dict) and page_info.get("hasNextPage") is False
                matched = [
                    node for node in (connection.get("nodes") or [])
                    if isinstance(node, dict) and cls._issue_uid(node)
                ] if isinstance(connection, dict) else []
                if complete:
                    issue_reads.append((request, matched))
                for node in matched:
                    identity = (node.get("number"), node.get("url") or node.get("html_url"))
                    all_issue_ids.add(identity)
                    if (
                        str(node.get("state") or "").lower() == "closed"
                        and str(node.get("stateReason") or node.get("state_reason") or "").lower() == "duplicate"
                    ):
                        exact_issue_observed = True

            if re.search(r"\bprojectV2\s*\(", query) or re.search(r"\bprojectItems\s*\(", query):
                complete = cls._project_connection_complete(payload, query)
                candidates = cls._candidate_items(payload)
                if complete:
                    project_reads.append((request, candidates))
                for candidate in candidates:
                    item = candidate["item"]
                    project = candidate["project"] or {}
                    item_identity = (item.get("id"), candidate["issue"].get("number"))
                    all_project_ids.add(item_identity)
                    fields = cls._item_fields(item)
                    if (
                        item.get("id") == "PVTI_candidate_fixture"
                        and candidate["issue"].get("number") == FIXTURE.CANDIDATE_ISSUE
                        and project.get("id") == "PVT_fixture"
                        and project.get("number") == 1
                        and ((project.get("owner") or {}).get("login") == "eng-cc")
                        and item.get("isArchived") is False
                        and fields == {
                            "Status": "Done",
                            "PM Status": "candidate",
                            "Workflow Phase": "bootstrap",
                            "Canonical Worktree": "",
                        }
                    ):
                        exact_project_observed = True

            # REST pagination is a complete Issue inventory only when every
            # slurped element is a page array. Direct Issue reads confirm state
            # but cannot establish UID uniqueness on their own.
            if cls._rest_pages_complete(read):
                page_records = [
                    record for page in payload for record in page
                    if isinstance(record, dict) and cls._issue_uid(record)
                ]
                issue_reads.append((request, page_records))
                for node in page_records:
                    identity = (node.get("number"), node.get("url") or node.get("html_url"))
                    all_issue_ids.add(identity)
                    if (
                        str(node.get("state") or "").lower() == "closed"
                        and str(node.get("stateReason") or node.get("state_reason") or "").lower() == "duplicate"
                    ):
                        exact_issue_observed = True

            # Record any direct REST Issue proof as exact state evidence. It is
            # not accepted as a uniqueness scan unless the full Issue list was
            # paginated above.
            if request.startswith("api repos/eng-cc/oasis7/issues/"):
                for node in cls._walk_dicts(payload):
                    if cls._issue_uid(node):
                        identity = (node.get("number"), node.get("url") or node.get("html_url"))
                        all_issue_ids.add(identity)
                        if (
                            str(node.get("state") or "").lower() == "closed"
                            and str(node.get("stateReason") or node.get("state_reason") or "").lower() == "duplicate"
                        ):
                            exact_issue_observed = True

        violations = []
        if not issue_reads:
            violations.append("no complete Issue discovery query was observed")
        elif any(len(matches) != 1 for _, matches in issue_reads):
            violations.append("complete Issue discovery did not return exactly one canonical UID match on every read")
        if len(all_issue_ids) != 1 or (None, None) in all_issue_ids:
            violations.append(f"observed matching Issue identities were missing or non-unique: {sorted(all_issue_ids, key=str)}")
        if not exact_issue_observed:
            violations.append("production-visible Issue proof lacked state=CLOSED and stateReason=DUPLICATE")
        if not project_reads:
            violations.append("no complete canonical Project item query was observed")
        elif any(len(matches) != 1 for _, matches in project_reads):
            violations.append("complete Project discovery did not return exactly one UID-bound item on every read")
        if len(all_project_ids) != 1 or ("PVTI_candidate_fixture", FIXTURE.CANDIDATE_ISSUE) not in all_project_ids:
            violations.append(f"observed UID-bound Project items were missing, foreign, or non-unique: {sorted(all_project_ids, key=str)}")
        if not exact_project_observed:
            violations.append("production-visible Project proof lacked canonical owner/number/archive/field identity")
        return violations

    @staticmethod
    def _refusal_text(result):
        def structured_text(value):
            semantic_keys = {"error", "message", "refusal", "reason", "blocker", "blockers", "next_action", "next_command"}
            found = []

            def collect(node, in_semantic_field=False):
                if isinstance(node, dict):
                    for key, child in node.items():
                        if key in semantic_keys:
                            collect(child, True)
                        else:
                            collect(child, in_semantic_field)
                elif isinstance(node, list):
                    for child in node:
                        collect(child, in_semantic_field)
                elif in_semantic_field and isinstance(node, str):
                    found.append(node)

            collect(value)
            return " ".join(str(item) for item in found)

        if result.stderr.strip():
            try:
                return structured_text(json.loads(result.stderr))
            except json.JSONDecodeError:
                return "\n".join(
                    line.strip() for line in result.stderr.splitlines()
                    if line.strip().lower().startswith((
                        "github-project-sync:",
                        "retire-closed-duplicate-candidate:",
                    ))
                )
        try:
            payload = json.loads(result.stdout)
        except (TypeError, json.JSONDecodeError):
            return ""
        return structured_text(payload)

    @staticmethod
    def _gh_call_class(call):
        if not call:
            return "unknown"
        if call[0] == "api":
            if len(call) > 1 and call[1] == "graphql":
                query = next((arg.split("=", 1)[1] for arg in call if arg.startswith("query=")), "")
                return "read" if query and "mutation" not in query.lower() else "write-or-unknown"
            method = next((call[index + 1].upper() for index, arg in enumerate(call[:-1]) if arg in ("-X", "--method")), "GET")
            if method in {"POST", "PUT", "PATCH", "DELETE"}:
                return "write-or-unknown"
            if any(arg in {"-f", "-F", "--input", "--raw-field"} for arg in call):
                return "write-or-unknown"
            return "read"
        if call[0] == "issue" and len(call) > 1:
            return "read" if call[1] in {"view", "list", "status"} else "write-or-unknown"
        if call[0] == "project" and len(call) > 1:
            return "read" if call[1] in {"view", "field-list", "item-list"} else "write-or-unknown"
        if call[0] == "pr" and len(call) > 1:
            return "read" if call[1] in {"view", "list", "status", "checks", "diff"} else "write-or-unknown"
        return "unknown"

    @staticmethod
    def _sync_mutations(calls):
        return [
            call for call in calls
            if ClosedDuplicateBootstrapGuardTests._gh_call_class(call.get("args", []) if isinstance(call, dict) else call) != "read"
        ]

    def test_global_sync_missing_candidate_mapping_row_rejects_before_mutation_with_recovery(self):
        self._write_sync_task()
        self._remove_candidate_mapping_row()
        before = self.fixture.mapping.read_bytes()

        result, calls, reads = self._run_global_sync(skip_recover=False)

        violations = self._live_duplicate_violations(reads)
        refusal = self._refusal_text(result).lower()
        if result.returncode == 0:
            violations.append("global sync returned success for a live closed-DUPLICATE UID absent from the mapping")
        if FIXTURE.CANDIDATE_UID not in refusal or not any(
            token in refusal for token in ("duplicate", "reconcile", "retire-closed-duplicate-candidate")
        ):
            violations.append("global sync did not identify the live closed-DUPLICATE candidate in its refusal")
        if self._sync_mutations(calls):
            violations.append(f"global sync attempted GitHub mutations: {self._sync_mutations(calls)}")
        if self.fixture.mapping.read_bytes() != before:
            violations.append("global sync changed mapping bytes after resolving the closed-DUPLICATE UID")
        self.assertEqual([], violations, "; ".join(violations) + f"\nstdout={result.stdout}\nstderr={result.stderr}")

    def test_global_sync_skip_recover_missing_candidate_mapping_row_rejects_before_mutation(self):
        self._write_sync_task()
        self._remove_candidate_mapping_row()
        before = self.fixture.mapping.read_bytes()

        result, calls, reads = self._run_global_sync(skip_recover=True)

        violations = self._live_duplicate_violations(reads)
        refusal = self._refusal_text(result).lower()
        if result.returncode == 0:
            violations.append("--skip-recover bypassed live closed-DUPLICATE UID discovery and returned success")
        if FIXTURE.CANDIDATE_UID not in refusal or not any(
            token in refusal for token in ("duplicate", "reconcile", "retire-closed-duplicate-candidate")
        ):
            violations.append("--skip-recover did not identify the live closed-DUPLICATE candidate in its refusal")
        if self._sync_mutations(calls):
            violations.append(f"--skip-recover attempted GitHub mutations: {self._sync_mutations(calls)}")
        if self.fixture.mapping.read_bytes() != before:
            violations.append("--skip-recover changed mapping bytes after selecting the closed-DUPLICATE UID")
        self.assertEqual([], violations, "; ".join(violations) + f"\nstdout={result.stdout}\nstderr={result.stderr}")

    def test_global_sync_still_creates_an_ordinary_new_task(self):
        new_uid = "task_" + "d" * 32
        self._write_sync_task(new_uid)
        before = self.fixture.mapping.read_bytes()

        result, calls, _reads = self._run_global_sync(skip_recover=True)

        mapping = json.loads(self.fixture.mapping.read_text(encoding="utf-8"))
        row = mapping["tasks"].get(new_uid, {})
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(1, json.loads(result.stdout)["created_issues"])
        self.assertEqual(1, json.loads(result.stdout)["added_items"])
        self.assertEqual("https://github.com/eng-cc/oasis7/issues/900099", row.get("issue_url"))
        self.assertNotEqual(before, self.fixture.mapping.read_bytes(), "valid task control must persist its new mapping")
        self.assertTrue(self._sync_mutations(calls), "valid new-task control must retain its normal GitHub writes")

    def test_bootstrap_refuses_live_closed_duplicate_alias_before_any_write(self):
        self.fixture._write_paginated_live_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate", (result.stderr + result.stdout).lower())
        self.assertIn("retire-closed-duplicate-candidate.py", (result.stderr + result.stdout))
        self.assertEqual(self.fixture.mapping.read_bytes(), before)
        self.assertFalse(self._snapshot_path().exists(), "rejected candidate bootstrap must not create a snapshot")

    def test_workflow_next_returns_reconcile_blocker_instead_of_bootstrap_command(self):
        self.fixture._write_paginated_live_gh_stub()

        result = self._run_workflow_next()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["next_action"], "blocked")
        self.assertEqual(payload["next_command"], [])
        self.assertTrue(any("duplicate" in blocker.lower() for blocker in payload["blockers"]))
        self.assertIn("retire-closed-duplicate-candidate.py", json.dumps(payload))

    def test_bootstrap_refuses_unaliased_live_closed_duplicate_before_snapshot(self):
        self._remove_foreign_owner_mapping()
        self.fixture._write_paginated_live_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate", (result.stderr + result.stdout).lower())
        self.assertIn("retire-closed-duplicate-candidate.py", result.stderr + result.stdout)
        self.assertEqual(self.fixture.mapping.read_bytes(), before)
        self.assertFalse(self._snapshot_path().exists(), "unaliased closed duplicate must not create a bootstrap snapshot")
        calls = [json.loads(line) for line in self.fixture.gh_log.read_text(encoding="utf-8").splitlines()]
        self.assertTrue(any(call[:1] == ["api"] for call in calls), "unaliased candidate must be checked against live Issue truth")

    def test_workflow_next_blocks_unaliased_live_closed_duplicate(self):
        self._remove_foreign_owner_mapping()
        self.fixture._write_paginated_live_gh_stub()

        result = self._run_workflow_next()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["next_action"], "blocked")
        self.assertEqual(payload["next_command"], [])
        self.assertTrue(any("duplicate" in blocker.lower() for blocker in payload["blockers"]))
        self.assertIn("retire-closed-duplicate-candidate.py", json.dumps(payload))

    def test_bootstrap_fails_closed_when_live_issue_authority_is_unavailable(self):
        self._write_unavailable_gh_stub()
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(self._snapshot_path().exists(), "unavailable live Issue proof must not create a snapshot")
        self.assertEqual(self.fixture.mapping.read_bytes(), before)

    def test_bootstrap_fails_closed_when_live_issue_uid_is_ambiguous(self):
        self.fixture._write_paginated_live_gh_stub()
        live = json.loads(self.fixture.gh_fixture.read_text(encoding="utf-8"))
        candidate = next(item for item in live["issues"] if item["number"] == FIXTURE.CANDIDATE_ISSUE)
        candidate["body"] += f"\ntask_uid: {FIXTURE.CANDIDATE_UID}\n"
        self.fixture.gh_fixture.write_text(json.dumps(live, sort_keys=True), encoding="utf-8")
        before = self.fixture.mapping.read_bytes()

        result = self._run_bootstrap()

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertFalse(self._snapshot_path().exists(), "ambiguous live Issue identity must not create a snapshot")
        self.assertEqual(self.fixture.mapping.read_bytes(), before)

    def test_existing_active_task_bootstrap_remains_compatible_without_candidate_guard(self):
        self._write_unavailable_gh_stub()
        if self.fixture.foreign_snapshot.exists():
            self.fixture.foreign_snapshot.unlink()

        result = self._run_bootstrap(FIXTURE.FOREIGN_UID)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.fixture.foreign_snapshot.is_file())
        self.assertFalse(self.fixture.gh_log.exists(), "active-task bootstrap must not enter candidate-only Issue guard")


if __name__ == "__main__":
    unittest.main(verbosity=2)
