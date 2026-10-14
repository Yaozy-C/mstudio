"""Exercise interrupted downloads, checksum failures and atomic cache updates."""
import hashlib
from http.client import IncompleteRead
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import URLError

import download_verified as downloader


class DownloadVerifiedTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.destination = self.root / 'sdk.zip'
        self.data = b'complete synthetic SDK archive'
        self.sha256 = hashlib.sha256(self.data).hexdigest()
        opener_patch = patch.object(downloader, 'urlopen')
        self.opener = opener_patch.start()
        self.addCleanup(opener_patch.stop)
        sleep_patch = patch.object(downloader.time, 'sleep')
        self.sleep = sleep_patch.start()
        self.addCleanup(sleep_patch.stop)

    def download(self, attempts=4):
        downloader.download_verified(
            'https://example.com/sdk.zip', self.destination, self.sha256, attempts,
        )

    def test_verified_cache_needs_no_network(self):
        self.destination.write_bytes(self.data)
        self.download()
        self.opener.assert_not_called()
        self.sleep.assert_not_called()

    def test_stream_failure_retries_and_publishes_only_verified_bytes(self):
        class Interrupted(io.BytesIO):
            def read(self, size=-1):
                if self.tell():
                    raise IncompleteRead(b'partial', 100)
                return super().read(size)

        self.opener.side_effect = [Interrupted(b'partial'), io.BytesIO(self.data)]
        self.download()
        self.assertEqual(self.destination.read_bytes(), self.data)
        self.assertEqual(self.opener.call_count, 2)
        self.sleep.assert_called_once_with(5)
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    def test_network_and_checksum_failures_retry_before_replacing_bad_cache(self):
        self.destination.write_bytes(b'old invalid cache')
        self.opener.side_effect = [
            URLError('connection reset'), io.BytesIO(b'truncated archive'),
            io.BytesIO(self.data),
        ]
        self.download()
        self.assertEqual(self.destination.read_bytes(), self.data)
        self.assertEqual(self.opener.call_count, 3)
        self.assertEqual([call.args[0] for call in self.sleep.call_args_list], [5, 10])
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    def test_exhausted_retries_preserve_existing_cache_and_remove_partial_files(self):
        for cached in [None, b'old invalid cache']:
            with self.subTest(cached=cached):
                if cached is not None:
                    self.destination.write_bytes(cached)
                self.opener.side_effect = lambda *args, **kwargs: io.BytesIO(b'bad')
                before = self.opener.call_count
                with self.assertRaisesRegex(RuntimeError, 'after 4 attempts'):
                    self.download()
                self.assertEqual(self.opener.call_count - before, 4)
                self.assertEqual(list(self.root.glob('*.part')), [])
                if cached is None:
                    self.assertFalse(self.destination.exists())
                else:
                    self.assertEqual(self.destination.read_bytes(), cached)


if __name__ == '__main__':
    unittest.main()
