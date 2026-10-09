#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
python3 - "$root" <<'PY'
import hashlib, os, pathlib, platform, subprocess, tempfile, unittest
root=pathlib.Path(__import__('sys').argv[1])
checksum='f2b4680cd239693a646a2795e4633c625328d7b2a044fbe749fa3a2fe9e7036b'
with tempfile.TemporaryDirectory(prefix='ci-trunk-proof-') as directory:
    fixture=pathlib.Path(tempfile.gettempdir())/('oasis7-trunk-test-'+checksum+'.tar.gz')
    if not fixture.is_file() or hashlib.sha256(fixture.read_bytes()).hexdigest()!=checksum:
        download=pathlib.Path(directory)/'original.tar.gz'
        subprocess.run(['curl','--fail','--location','--silent','--show-error','--http1.1','--retry','3','--connect-timeout','15','--max-time','120',
            'https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz',
            '--output',str(download)],check=True)
        assert hashlib.sha256(download.read_bytes()).hexdigest()==checksum
        os.replace(download,fixture)
    assert hashlib.sha256(fixture.read_bytes()).hexdigest()==checksum
    members=subprocess.check_output(['tar','-tzf',str(fixture)],text=True).splitlines()
    assert 'trunk' in members
    class Install(unittest.TestCase):
        def setUp(self):
            self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
            self.work=pathlib.Path(self.temp.name);spies=self.work/'spies';spies.mkdir()
            scripts={
                'uname':'#!/bin/bash\nif [[ "$1" == -s ]]; then echo Linux; else echo x86_64; fi\n',
                'curl':'#!/bin/bash\nset -eu\nprintf "download\\n" >>"$TEST_LOG"\n[[ "${TEST_DOWNLOAD_FAIL:-0}" == 0 ]] || exit 73\nwhile [[ "$1" != --output ]]; do shift; done\ncp "$TEST_ARCHIVE" "$2"\n',
                'tar':'#!/bin/bash\nset -eu\n[[ "$1" == -xzf && "$3" == -C && "$5" == trunk ]]\nprintf "extract\\n" >>"$TEST_LOG"\nprintf "#!/bin/bash\\necho \'trunk %s\'\\n" "${TEST_VERSION:-0.21.14}" >"$4/trunk"\n'}
            for name,source in scripts.items():
                path=spies/name;path.write_text(source);path.chmod(0o755)
            self.archive=self.work/'cache'/'trunk.tar.gz';self.bin=self.work/'bin';self.log=self.work/'calls'
            self.env=dict(os.environ,PATH=str(spies)+os.pathsep+os.environ['PATH'],TEST_ARCHIVE=str(fixture),TEST_LOG=str(self.log),TRUNK_VERSION='0.21.14',GITHUB_PATH=str(self.work/'github-path'))
        def run_install(self,success=True):
            result=subprocess.run(['bash',str(root/'scripts/install-ci-trunk.sh'),str(self.archive),str(self.bin)],env=self.env,text=True,capture_output=True)
            self.assertEqual(result.returncode==0,success,result.stderr)
            if not success:self.assertFalse((self.bin/'trunk').exists())
            return result
        def test_download_and_valid_cache_both_verify_before_extract(self):
            self.run_install();self.assertEqual(self.log.read_text().splitlines(),['download','extract'])
            self.assertEqual(hashlib.sha256(self.archive.read_bytes()).hexdigest(),checksum)
            (self.bin/'trunk').unlink();self.log.write_text('');self.run_install()
            self.assertEqual(self.log.read_text().splitlines(),['extract'])
            self.assertEqual((self.work/'github-path').read_text().splitlines(),[str(self.bin),str(self.bin)])
        def test_corrupt_restore_never_extracts_or_executes(self):
            self.archive.parent.mkdir();self.archive.write_bytes(b'corrupted cache')
            self.run_install(False);self.assertFalse(self.log.exists())
        def test_failed_download_does_not_publish_archive(self):
            self.env['TEST_DOWNLOAD_FAIL']='1';self.run_install(False);self.assertFalse(self.archive.exists())
        def test_corrupt_download_never_extracts(self):
            corrupt=self.work/'corrupt';corrupt.write_bytes(b'wrong release bytes');self.env['TEST_ARCHIVE']=str(corrupt)
            self.run_install(False);self.assertEqual(self.log.read_text().splitlines(),['download']);self.assertFalse(self.archive.exists())
        def test_wrong_version_never_installs(self):
            self.env['TEST_VERSION']='999.0.0';self.run_install(False);self.assertEqual(self.log.read_text().splitlines(),['download','extract'])
        def test_unsupported_pin_never_downloads(self):
            self.env['TRUNK_VERSION']='999.0.0';self.run_install(False);self.assertFalse(self.log.exists())
        @unittest.skipUnless(platform.system()=='Linux' and platform.machine()=='x86_64','real Linux binary requires its supported host')
        def test_real_release_binary_on_supported_host(self):
            self.archive.parent.mkdir();self.archive.write_bytes(fixture.read_bytes())
            self.env['PATH']=os.environ['PATH'];self.run_install()
            self.assertEqual(subprocess.check_output([str(self.bin/'trunk'),'--version'],text=True).strip(),'trunk 0.21.14')
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(Install)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():raise SystemExit(1)
PY
