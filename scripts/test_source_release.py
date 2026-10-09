"""Regression checks for narrowly allowed release assets and documents."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest


class SourceReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        (self.root / 'scripts').mkdir()
        shutil.copy(Path(__file__).with_name('check_source_release.py'), self.root / 'scripts')
        self.effects = self.root / 'frontend/public/effects'
        self.effects.mkdir(parents=True)
        self.video = self.effects / 'demo.mp4'
        self.video.write_bytes(b'preview' * 320000)
        self.record()

    def record(self):
        data = self.video.read_bytes()
        (self.effects / 'sources.json').write_text(json.dumps([{
            'file': self.video.name, 'bytes': len(data),
            'sha256': hashlib.sha256(data).hexdigest(),
            'sourcePage': 'https://example.com/demo',
            'sourceVideo': 'https://example.com/demo.mp4',
            'usage': 'Synthetic test data',
        }]))

    def check_release(self):
        return subprocess.run([sys.executable, str(self.root / 'scripts/check_source_release.py')],
                              capture_output=True, text=True)

    def test_pinned_preview_over_default_size_passes(self):
        result = self.check_release()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_generated_transition_previews_require_pinned_manifest(self):
        target = self.effects.with_name('transition-previews')
        self.effects.rename(target)
        self.assertEqual(self.check_release().returncode, 0)
        (target / 'demo.mp4').write_bytes(b'changed')
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_unlisted_media_remains_blocked(self):
        (self.effects / 'unlisted.mp4').write_bytes(b'private footage')
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_changed_preview_is_blocked(self):
        self.video.write_bytes(b'changed preview')
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_manifest_cannot_raise_preview_limit(self):
        self.video.write_bytes(b'x' * (3 * 1024 * 1024 + 1))
        self.record()
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_pinning_does_not_bypass_content_scan(self):
        self.video.write_bytes(b'-----BEGIN ' + b'PRIVATE KEY-----')
        self.record()
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_public_case_notes_pass_but_unlisted_research_stays_blocked(self):
        docs = self.root / 'docs'
        docs.mkdir()
        (docs / 'image-storyboard-cases.md').write_text('# Public storyboard cases\n')
        result = self.check_release()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        (docs / 'private-research.md').write_text('Internal notes\n')
        self.assertNotEqual(self.check_release().returncode, 0)

    def test_public_case_notes_still_receive_content_scan(self):
        docs = self.root / 'docs'
        docs.mkdir()
        (docs / 'image-storyboard-cases.md').write_text('-----BEGIN ' + 'PRIVATE KEY-----')
        self.assertNotEqual(self.check_release().returncode, 0)


if __name__ == '__main__':
    unittest.main()
