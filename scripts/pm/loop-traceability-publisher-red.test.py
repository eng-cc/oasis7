#!/usr/bin/env python3
"""Focused contract for the comment-ID-bound coordinating-record publisher.

``FakeGitHub`` models only the live Issue/comment operations needed to prove
the protocol: durable intent before the first POST, inert reservation recovery,
same-ID finalization, and exact server readback. It never treats a reservation
as a traceability record.
"""

from copy import deepcopy
from contextlib import redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
PUBLISHER_LAUNCHER = HERE / "publish-traceability-record.sh"
TRACEABILITY_SCRIPT = HERE / "loop_traceability.py"
REPOSITORY = "eng-cc/oasis7"
ISSUE_NUMBER = 3913
TASK_UID = "task_" + "d" * 32
CHANGE_ID = "change-q1-traceability"
SOURCE_OID = "a" * 40
RESERVATION_ID = 94001
GUESSED_ID = 94000
NONCE = "publication-nonce-q1"
ISSUE_URL = f"https://github.com/{REPOSITORY}/issues/{ISSUE_NUMBER}"
API_ISSUE_URL = f"https://api.github.com/repos/{REPOSITORY}/issues/{ISSUE_NUMBER}"
RESERVATION_MARKER = "oasis7-loop-change-reservation"
RESERVATION_SCHEMA = "oasis7.loop-change-publication-reservation/v1"


def _load_traceability():
    path = HERE / "loop_traceability.py"
    spec = importlib.util.spec_from_file_location("loop_traceability_publisher_red", path)
    if spec is None or spec.loader is None:
        raise AssertionError("loop_traceability.py has no import loader")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TRACE = _load_traceability()


def _draft_record():
    """Return the protocol-only draft input, which has no published comment ID.

    This intentionally is not a published record: the publisher must bind the
    GitHub-assigned ID and compute the final record digest before emitting the
    canonical record body.
    """
    return {
        "schema": TRACE.SCHEMA,
        "marker": TRACE.MARKER,
        "task_uid": TASK_UID,
        "change_id": CHANGE_ID,
        "coordination_ref": {
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "source_commit": SOURCE_OID,
            "record_digest": "",
        },
        "required_obligations": [{
            "obligation_id": "Q1-PUBLISHER",
            "mapping_slot": "slot-publisher",
            "required": True,
            "acceptance_refs": [{
                "repository": REPOSITORY,
                "path": "doc/engineering/workflow/source-of-truth.md",
                "fragment": "traceability-record-contract",
            }],
            "owner_loop": "code",
            "owner_role": "repository_health_engineer",
        }],
        "mapping_slots": [{
            "slot_id": "slot-publisher",
            "owner_loop": "code",
            "owner_role": "repository_health_engineer",
        }],
        "candidate_selection": {"comparable_fields": list(TRACE.CANDIDATE_FIELDS)},
        "feedback": [],
    }


def _draft_intent(record=None):
    draft = _draft_record() if record is None else record
    body = TRACE.canonical_bytes(draft).decode("utf-8")
    return {
        "repository": REPOSITORY,
        "issue_number": ISSUE_NUMBER,
        "task_uid": TASK_UID,
        "change_id": CHANGE_ID,
        "draft_body": body,
        "draft_digest": TRACE.canonical_digest(draft),
    }


def _reservation_body(nonce=NONCE, draft_digest=None):
    return TRACE.canonical_bytes({
        "marker": RESERVATION_MARKER,
        "schema": RESERVATION_SCHEMA,
        "repository": REPOSITORY,
        "issue_number": ISSUE_NUMBER,
        "task_uid": TASK_UID,
        "change_id": CHANGE_ID,
        "nonce": nonce,
        "draft_digest": draft_digest or _draft_intent()["draft_digest"],
    }).decode("utf-8")


def _comment(comment_id, body, *, issue_url=API_ISSUE_URL, author="publisher"):
    return {
        "id": comment_id,
        "body": body,
        "issue_url": issue_url,
        "user": {"login": author},
        "created_at": "2026-09-23T00:00:00Z",
    }


class DurableIntent:
    """A small journal fake that returns the same persisted nonce on retry."""

    def __init__(self, saved=None):
        self.saved = deepcopy(saved)
        self.calls = 0
        self.events = []

    def __call__(self, *args, **kwargs):
        self.calls += 1
        self.events.append("intent")
        identity = next(
            (arg for arg in args if isinstance(arg, dict) and "draft_body" in arg),
            _draft_intent(),
        )
        created = self.saved is None
        if self.saved is None:
            self.saved = {**deepcopy(identity), "nonce": NONCE}
        elif {key: self.saved.get(key) for key in identity} != identity:
            raise AssertionError("retry attempted to change a persisted publication intent")
        return {**deepcopy(self.saved), "newly_created": created}


class FakeGitHub:
    """Deterministic Issue/comment API with injectable uncertain writes."""

    def __init__(self):
        self.events = []
        self.comments = []
        self.posted_bodies = []
        self.complete_pagination = True
        self.current_login = "publisher"
        self.current_permission = "admin"
        self.permission_lookups = []
        self.next_comment_id = RESERVATION_ID
        self.lose_post_response = False
        self.lose_patch_response = False
        self.lose_get_response = False
        self.patch_without_effect_once = False
        self.wrong_final_body = False
        self.wrong_issue_url = False
        self.wrong_comment_issue_url = False
        self.issue_html_url = ISSUE_URL
        self.user_response = None
        self.permission_response = None
        self.duplicate_final_after_patch = False
        self.incomplete_after_patch = False
        self.issue_number = ISSUE_NUMBER
        self.issue_task_uid = TASK_UID
        self.publication_intent = DurableIntent()

    def before_write(self, identity):
        return self.publication_intent(identity)

    def issue(self, issue_number):
        self.events.append("issue")
        return {
            "number": self.issue_number,
            "html_url": "https://github.com/foreign/repo/issues/1" if self.wrong_issue_url else self.issue_html_url,
            "body": f"<!-- oasis7-pm-task -->\ntask_uid: {self.issue_task_uid}\n",
        }

    def user(self):
        self.events.append("user")
        if self.user_response is not None:
            return deepcopy(self.user_response)
        return {"login": self.current_login}

    def permission(self, login):
        self.events.append(("permission", login))
        self.permission_lookups.append(login)
        if self.permission_response is not None:
            return deepcopy(self.permission_response)
        return {"permission": self.current_permission}

    def discover_comments(self, issue_number):
        self.events.append("discover")
        if issue_number != self.issue_number:
            return {"complete": True, "comments": []}
        return {"complete": self.complete_pagination, "comments": deepcopy(self.comments)}

    def post_reservation(self, issue_number, body):
        self.events.append("post")
        self.assert_issue(issue_number)
        self.posted_bodies.append(body)
        comment = _comment(self.next_comment_id, body, author=self.current_login)
        self.next_comment_id += 1
        self.comments.append(comment)
        if self.lose_post_response:
            self.lose_post_response = False
            raise TimeoutError("simulated lost reservation POST response")
        return deepcopy(comment)

    def get_comment(self, issue_number, comment_id):
        self.events.append(("get", comment_id))
        self.assert_issue(issue_number)
        if self.lose_get_response:
            self.lose_get_response = False
            raise TimeoutError("simulated uncertain exact comment read")
        for comment in self.comments:
            if comment["id"] == comment_id:
                result = deepcopy(comment)
                if self.wrong_comment_issue_url:
                    result["issue_url"] = "https://api.github.com/repos/foreign/repo/issues/1"
                if self.wrong_final_body and json.loads(result["body"]).get("marker") == TRACE.MARKER:
                    result["body"] += " "
                return result
        raise ValueError("comment not found")

    def patch_comment(self, issue_number, comment_id, body):
        self.events.append(("patch", comment_id))
        self.assert_issue(issue_number)
        if self.patch_without_effect_once:
            self.patch_without_effect_once = False
            raise TimeoutError("simulated PATCH timeout before write")
        for comment in self.comments:
            if comment["id"] == comment_id:
                comment["body"] = body
                if self.duplicate_final_after_patch:
                    duplicate = deepcopy(comment)
                    duplicate["id"] = self.next_comment_id
                    self.next_comment_id += 1
                    self.comments.append(duplicate)
                    self.duplicate_final_after_patch = False
                if self.incomplete_after_patch:
                    self.complete_pagination = False
                    self.incomplete_after_patch = False
                if self.lose_patch_response:
                    self.lose_patch_response = False
                    raise TimeoutError("simulated lost PATCH response")
                return deepcopy(comment)
        raise ValueError("comment not found")

    def assert_issue(self, issue_number):
        if issue_number != self.issue_number:
            raise ValueError("wrong Issue")

    def add_reservation(self, comment_id, *, nonce=NONCE, body=None, author="publisher"):
        self.comments.append(_comment(
            comment_id,
            body if body is not None else _reservation_body(nonce),
            author=author,
        ))


class TraceabilityPublisherTests(unittest.TestCase):
    def setUp(self):
        self.record = _draft_record()
        self.adapter = FakeGitHub()
        self.intent = DurableIntent()

    def publish(self, record=None):
        publisher = getattr(TRACE, "publish_traceability_record", None)
        self.assertTrue(
            callable(publisher),
            "missing producer behavior: loop_traceability.publish_traceability_record",
        )
        return publisher(
            deepcopy(self.record if record is None else record),
            self.adapter,
            before_write=self.intent,
        )

    def assert_blocked(self, operation):
        with self.assertRaises((TRACE.TraceabilityError, ValueError, RuntimeError)):
            operation()

    def _launcher_fixture(self, root: Path):
        repo = root
        pm = repo / "scripts" / "pm"
        pm.mkdir(parents=True)
        launcher = pm / PUBLISHER_LAUNCHER.name
        shutil.copy2(PUBLISHER_LAUNCHER, launcher)
        invocation_log = repo / "launcher-invocation.json"
        fake_publisher = pm / "loop_traceability.py"
        fake_publisher.write_text(
            "import json\n"
            "import sys\n"
            "from pathlib import Path\n"
            "def _trusted_launcher_main(argv):\n"
            f"    Path({str(invocation_log)!r}).write_text(json.dumps({{\n"
            "        'argv': argv, 'trusted_launcher': True,\n"
            "        'isolated': sys.flags.isolated, 'no_site': sys.flags.no_site,\n"
            "    }))\n"
            "    return 0\n",
            encoding="utf-8",
        )
        subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
        subprocess.run(["git", "-C", str(repo), "add", "scripts/pm"], check=True)
        subprocess.run(
            [
                "git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.com",
                "commit", "-q", "-m", "trusted publisher fixture",
            ],
            check=True,
        )
        return repo, launcher, invocation_log, pm

    def test_launcher_rejects_untracked_import_shadow_before_python_startup(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-launcher-shadow-") as directory:
            repo, launcher, invocation_log, pm = self._launcher_fixture(Path(directory))
            draft = repo / "draft.json"
            draft.write_bytes(b"{}")
            (repo / ".gitignore").write_text("scripts/pm/json.py\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", ".gitignore"], check=True)
            subprocess.run(
                [
                    "git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.com",
                    "commit", "-q", "-m", "ignore the shadow path",
                ],
                check=True,
            )
            shadow_ran = repo / "shadow-ran.txt"
            (pm / "json.py").write_text(
                f"open({str(shadow_ran)!r}, 'w').write('executed')\n",
                encoding="utf-8",
            )

            result = subprocess.run(
                [str(launcher), "--draft", str(draft), "--enable-publication"],
                cwd=repo,
                text=True,
                capture_output=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("untracked", result.stderr.lower())
            self.assertFalse(shadow_ran.exists(), "untracked import shadow ran before the trust gate")
            self.assertFalse(invocation_log.exists(), "Python publisher started before the trust gate")

    def test_launcher_allows_exact_untracked_draft_outside_import_root(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-launcher-draft-") as directory:
            repo, launcher, invocation_log, pm = self._launcher_fixture(Path(directory))
            draft = repo / "draft.json"
            draft.write_bytes(b"{}")

            # Keep this explicit even when the local Python runtime happens to
            # default to dont_write_bytecode: the launcher must not create an
            # untracked import root that its next clean preflight rejects.
            launcher_text = launcher.read_text(encoding="utf-8")
            self.assertIn("exec python3 -B -I -S -c", launcher_text)

            for attempt in range(2):
                with self.subTest(attempt=attempt + 1):
                    result = subprocess.run(
                        [str(launcher), "--draft", str(draft)],
                        cwd=repo,
                        text=True,
                        capture_output=True,
                    )

                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertFalse(
                        (pm / "__pycache__").exists(),
                        "publisher startup created bytecode in the guarded import root",
                    )
                    invocation = json.loads(invocation_log.read_text(encoding="utf-8"))
                    self.assertEqual(invocation["argv"][:3], ["publish-record", "--draft", str(draft.resolve())])
                    self.assertTrue(invocation["trusted_launcher"])
                    self.assertEqual(invocation["isolated"], 1)
                    self.assertEqual(invocation["no_site"], 1)

    def test_direct_python_publish_record_refuses_before_untracked_import_shadow(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-direct-python-") as directory:
            repo = Path(directory)
            pm = repo / "scripts" / "pm"
            pm.mkdir(parents=True)
            script = pm / "loop_traceability.py"
            shutil.copyfile(TRACEABILITY_SCRIPT, script)
            shadow_ran = repo / "shadow-ran.txt"
            (pm / "json.py").write_text(
                f"open({str(shadow_ran)!r}, 'w').write('executed')\n",
                encoding="utf-8",
            )
            draft = repo / "draft.json"
            draft.write_bytes(b"{}")

            result = subprocess.run(
                [
                    "python3", str(script), "publish-record", "--draft", str(draft),
                    "--repo-root", str(repo), "--enable-publication",
                ],
                cwd=repo,
                text=True,
                capture_output=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("publish-traceability-record.sh", result.stderr)
            self.assertFalse(shadow_ran.exists(), "direct Python path executed untracked import code")
            self.assertFalse((repo / ".git" / "oasis7-traceability-publications").exists())

    def test_direct_production_api_requires_launcher_before_api_or_journal(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-direct-api-") as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            publisher = TRACE.GitHubTraceabilityPublisher(repo)
            api_calls = []
            journal = repo / ".git" / "oasis7-traceability-publications"
            draft_path = repo / "draft.json"
            draft_path.write_bytes(TRACE.canonical_bytes(self.record))
            original_run = subprocess.run

            def record_api_call(*args, **kwargs):
                api_calls.append((args, kwargs))
                raise AssertionError("unexpected GitHub subprocess before launcher authorization")

            subprocess.run = record_api_call
            try:
                with self.assertRaisesRegex(TRACE.TraceabilityError, "trusted isolated operational launcher"):
                    TRACE.publish_traceability_record(
                        self.record,
                        publisher,
                        before_write=publisher.before_write,
                        enable_publication=True,
                    )
                with self.assertRaisesRegex(TRACE.TraceabilityError, "trusted isolated operational launcher"):
                    publisher.post_reservation(ISSUE_NUMBER, "not-authorized")
                with self.assertRaisesRegex(TRACE.TraceabilityError, "trusted isolated operational launcher"):
                    publisher.patch_comment(ISSUE_NUMBER, RESERVATION_ID, "not-authorized")

                output = io.StringIO()
                with redirect_stdout(output):
                    exit_code = TRACE.main(
                        [
                            "publish-record", "--draft", str(draft_path), "--repo-root", str(repo),
                            "--enable-publication",
                        ],
                        publisher_factory=lambda _root: publisher,
                    )
            finally:
                subprocess.run = original_run

            self.assertEqual(api_calls, [])
            self.assertFalse(journal.exists())
            self.assertEqual(exit_code, 2)
            self.assertIn("trusted isolated operational launcher", json.loads(output.getvalue())["error"])
            self.assertEqual(api_calls, [])
            self.assertFalse(journal.exists())

    def test_first_publication_binds_server_assigned_id_and_exact_canonical_body(self):
        result = self.publish()

        self.assertEqual(self.intent.calls, 1)
        self.assertLess(self.intent.events.index("intent"), self.adapter.events.index("post"))
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 1)
        self.assertGreaterEqual(len(self.adapter.permission_lookups), 2)
        self.assertEqual(set(self.adapter.permission_lookups), {"publisher"})
        final = self.adapter.get_comment(ISSUE_NUMBER, RESERVATION_ID)
        final_record = json.loads(final["body"])["record"]
        self.assertEqual(final_record["coordination_ref"]["comment_id"], RESERVATION_ID)
        self.assertEqual(
            final_record["coordination_ref"]["record_digest"],
            TRACE.record_digest(final_record),
        )
        expected_payload = {
            "marker": TRACE.MARKER,
            "schema": TRACE.SCHEMA,
            "task_uid": final_record["task_uid"],
            "change_id": final_record["change_id"],
            "record_digest": final_record["coordination_ref"]["record_digest"],
            "record": final_record,
        }
        self.assertEqual(final["body"], TRACE.canonical_bytes(expected_payload).decode("utf-8"))
        self.assertEqual(result["issue_number"], ISSUE_NUMBER)
        self.assertEqual(result["comment_id"], RESERVATION_ID)

        self.assertEqual(len(self.adapter.posted_bodies), 1)
        posted_reservation = json.loads(self.adapter.posted_bodies[0])
        expected_reservation = {
            "marker": RESERVATION_MARKER,
            "schema": RESERVATION_SCHEMA,
            "repository": REPOSITORY,
            "issue_number": ISSUE_NUMBER,
            "task_uid": TASK_UID,
            "change_id": CHANGE_ID,
            "nonce": NONCE,
            "draft_digest": self.intent.saved["draft_digest"],
        }
        self.assertEqual(
            self.adapter.posted_bodies[0],
            TRACE.canonical_bytes(expected_reservation).decode("utf-8"),
        )
        self.assertEqual(posted_reservation, expected_reservation)

    def test_persisted_draft_digest_and_nonce_are_inert_reservation_identity(self):
        self.publish()
        reservation = json.loads(self.adapter.posted_bodies[0])
        self.assertEqual(reservation["marker"], RESERVATION_MARKER)
        self.assertEqual(reservation["nonce"], self.intent.saved["nonce"])
        self.assertEqual(reservation["draft_digest"], self.intent.saved["draft_digest"])
        self.assertNotIn("record", reservation)

    def test_uncertain_post_recovery_uses_the_unique_matching_reservation_without_repost(self):
        self.adapter.lose_post_response = True
        try:
            self.publish()
        except (TRACE.TraceabilityError, ValueError, TimeoutError):
            pass

        # Resume with a fresh process-local journal object loaded from the
        # durable intent captured by the first process.
        self.intent = DurableIntent(saved=self.intent.saved)
        result = self.publish()
        self.assertEqual(result["comment_id"], RESERVATION_ID)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(len(self.adapter.comments), 1)
        self.assertEqual(self.intent.saved["nonce"], NONCE)
        self.assertEqual(self.intent.saved["draft_body"], _draft_intent()["draft_body"])

    def test_uncertain_post_with_incomplete_or_empty_discovery_stays_pending_without_repost(self):
        for label, prepare in (
            ("incomplete", lambda: setattr(self.adapter, "complete_pagination", False)),
            ("empty", lambda: self.adapter.comments.clear()),
        ):
            with self.subTest(case=label):
                self.adapter = FakeGitHub()
                self.intent = DurableIntent()
                self.adapter.lose_post_response = True
                try:
                    self.publish()
                except (TRACE.TraceabilityError, ValueError, TimeoutError):
                    pass
                prepare()
                self.assert_blocked(self.publish)
                self.assertEqual(self.adapter.events.count("post"), 1)

    def test_duplicate_or_mismatched_reservation_identity_stays_pending(self):
        self.adapter.add_reservation(RESERVATION_ID)
        self.adapter.add_reservation(RESERVATION_ID + 1)
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 0)

        self.adapter = FakeGitHub()
        self.adapter.add_reservation(RESERVATION_ID, nonce="stale-nonce")
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

    def test_prepopulated_guessed_or_stale_comment_id_is_not_publication_authority(self):
        guessed = deepcopy(self.record)
        guessed["coordination_ref"]["comment_id"] = GUESSED_ID
        guessed["coordination_ref"]["record_digest"] = TRACE.record_digest(guessed)
        self.adapter.comments.append(_comment(GUESSED_ID, "{}"))

        self.assert_blocked(lambda: self.publish(guessed))
        self.assertEqual(self.adapter.events.count("post"), 0)
        self.assertEqual(self.adapter.events.count(("patch", GUESSED_ID)), 0)

    def test_wrong_issue_task_or_reservation_author_blocks_before_final_patch(self):
        self.adapter.issue_task_uid = "task_" + "e" * 32
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

        self.adapter = FakeGitHub()
        self.adapter.wrong_issue_url = True
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

        self.adapter = FakeGitHub()
        self.adapter.current_permission = "read"
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

        self.adapter = FakeGitHub()
        self.adapter.user_response = {"login": "publisher"}
        self.adapter.permission_response = {}
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

        self.adapter = FakeGitHub()
        self.adapter.user_response = {"login": None}
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

        self.adapter = FakeGitHub()
        self.adapter.current_login = "different-publisher"
        self.adapter.add_reservation(RESERVATION_ID, author="publisher")
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 0)

        self.adapter = FakeGitHub()
        self.adapter.add_reservation(RESERVATION_ID)
        self.adapter.comments[0]["user"] = []
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 0)

    def test_uncertain_exact_reservation_get_stays_pending_without_patch(self):
        self.adapter.lose_get_response = True
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 0)

    def test_uncertain_patch_exact_final_readback_recovers_without_second_patch(self):
        self.adapter.lose_patch_response = True
        try:
            self.publish()
        except (TRACE.TraceabilityError, ValueError, TimeoutError):
            pass

        result = self.publish()
        self.assertEqual(result["comment_id"], RESERVATION_ID)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 1)

    def test_uncertain_patch_retries_only_same_id_when_exact_reservation_remains(self):
        self.adapter.patch_without_effect_once = True
        try:
            self.publish()
        except (TRACE.TraceabilityError, ValueError, TimeoutError):
            pass

        result = self.publish()
        self.assertEqual(result["comment_id"], RESERVATION_ID)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 2)
        self.assertFalse(any(event == ("patch", GUESSED_ID) for event in self.adapter.events))

    def test_wrong_body_duplicate_final_marker_and_noncanonical_readback_block(self):
        self.adapter.wrong_final_body = True
        self.assert_blocked(self.publish)

        self.adapter = FakeGitHub()
        self.adapter.add_reservation(RESERVATION_ID, body=TRACE.canonical_bytes({
            "marker": TRACE.MARKER,
            "schema": TRACE.SCHEMA,
            "task_uid": TASK_UID,
            "change_id": CHANGE_ID,
            "record_digest": "sha256:" + "0" * 64,
            "record": {},
        }).decode("utf-8"))
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 0)

    def test_wrong_comment_issue_url_blocks_publication_readback(self):
        self.adapter.wrong_comment_issue_url = True
        self.assert_blocked(self.publish)

    def test_duplicate_active_marker_after_patch_blocks_authority(self):
        self.adapter.duplicate_final_after_patch = True
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 1)

    def test_incomplete_final_pagination_blocks_authority(self):
        self.adapter.incomplete_after_patch = True
        self.assert_blocked(self.publish)
        self.assertEqual(self.adapter.events.count("post"), 1)
        self.assertEqual(self.adapter.events.count(("patch", RESERVATION_ID)), 1)

    def test_incomplete_initial_pagination_never_authorizes_first_post(self):
        self.adapter.complete_pagination = False
        self.assert_blocked(self.publish)
        self.assertEqual(self.intent.calls, 0)
        self.assertEqual(self.adapter.events.count("post"), 0)

    def test_production_intent_journal_survives_adapter_restart_without_changing_draft(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-intent-") as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            identity = _draft_intent()
            first = TRACE.GitHubTraceabilityPublisher(repo).before_write(identity)
            resumed = TRACE.GitHubTraceabilityPublisher(repo).before_write(identity)

            self.assertTrue(first["newly_created"])
            self.assertFalse(resumed["newly_created"])
            self.assertEqual(first["nonce"], resumed["nonce"])
            self.assertEqual(first["draft_body"], identity["draft_body"])
            self.assertEqual(first["draft_digest"], identity["draft_digest"])

            changed = {**identity, "draft_body": identity["draft_body"] + " "}
            with self.assertRaises(TRACE.TraceabilityError):
                TRACE.GitHubTraceabilityPublisher(repo).before_write(changed)

    def test_production_publication_requires_explicit_opt_in_and_postmerge_default_head(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-enable-") as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
            subprocess.run(
                [
                    "git", "-C", str(repo), "-c", "user.name=Test", "-c", "user.email=test@example.com",
                    "commit", "--allow-empty", "-q", "-m", "test base",
                ],
                check=True,
            )
            default_oid = subprocess.run(
                ["git", "-C", str(repo), "rev-parse", "main"], check=True, text=True, capture_output=True,
            ).stdout.strip()
            # An untracked local draft is an input, not publisher code.
            (repo / "draft.json").write_bytes(TRACE.canonical_bytes(self.record))

            class OfflinePublisher(TRACE.GitHubTraceabilityPublisher):
                def api(self, path, **_kwargs):
                    if path == f"repos/{REPOSITORY}":
                        return {"default_branch": "main"}
                    if path == f"repos/{REPOSITORY}/commits/main":
                        return {"sha": default_oid}
                    raise AssertionError(f"unexpected API request before publication gate: {path}")

            publisher = OfflinePublisher(repo)
            publisher.require_postmerge_enablement()
            with self.assertRaises(TRACE.TraceabilityError):
                TRACE.publish_traceability_record(self.record, publisher)

            subprocess.run(["git", "-C", str(repo), "switch", "-q", "-c", "candidate"], check=True)
            with self.assertRaisesRegex(TRACE.TraceabilityError, "live default-branch HEAD"):
                publisher.require_postmerge_enablement()
            self.assertFalse((repo / ".git" / "oasis7-traceability-publications").exists())

    def test_cli_default_preflight_does_not_create_intent_or_write_comments(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-cli-") as directory:
            subprocess.run(["git", "init", "-q", str(directory)], check=True)
            draft_path = Path(directory) / "draft.json"
            draft_path.write_bytes(TRACE.canonical_bytes(self.record))
            live = FakeGitHub()

            class ReadOnlyProductionAdapter(TRACE.GitHubTraceabilityPublisher):
                def issue(self, issue_number):
                    return live.issue(issue_number)

                def user(self):
                    return live.user()

                def permission(self, login):
                    return live.permission(login)

                def discover_comments(self, issue_number):
                    return live.discover_comments(issue_number)

                def get_comment(self, issue_number, comment_id):
                    return live.get_comment(issue_number, comment_id)

                def post_reservation(self, issue_number, body):
                    return live.post_reservation(issue_number, body)

                def patch_comment(self, issue_number, comment_id, body):
                    return live.patch_comment(issue_number, comment_id, body)

            adapter = ReadOnlyProductionAdapter(directory)
            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = TRACE.main(
                    ["publish-record", "--draft", str(draft_path), "--repo-root", directory],
                    publisher_factory=lambda _root: adapter,
                )

            self.assertEqual(exit_code, 0)
            self.assertEqual(json.loads(output.getvalue())["status"], "preflight_passed")
            self.assertFalse((Path(directory) / ".git" / "oasis7-traceability-publications").exists())
            self.assertEqual(live.events.count("post"), 0)
            self.assertEqual(live.events.count(("patch", RESERVATION_ID)), 0)

    def test_cli_mutation_requires_explicit_enablement_and_emits_only_receipt(self):
        with tempfile.TemporaryDirectory(prefix="oasis7-publisher-cli-enabled-") as directory:
            draft_path = Path(directory) / "draft.json"
            draft_path.write_bytes(TRACE.canonical_bytes(self.record))
            adapter = FakeGitHub()
            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = TRACE.main(
                    [
                        "publish-record", "--draft", str(draft_path), "--repo-root", directory,
                        "--enable-publication",
                    ],
                    publisher_factory=lambda _root: adapter,
                )

            receipt = json.loads(output.getvalue())
            self.assertEqual(exit_code, 0)
            self.assertEqual(set(receipt), {
                "repository", "issue_number", "comment_id", "task_uid", "change_id",
                "record_digest", "body_digest",
            })
            self.assertEqual(adapter.events.count("post"), 1)
            self.assertEqual(adapter.events.count(("patch", RESERVATION_ID)), 1)


if __name__ == "__main__":
    unittest.main()
