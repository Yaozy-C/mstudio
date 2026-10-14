"""Download pinned build dependencies without accepting incomplete cache files."""
import argparse
import hashlib
from http.client import HTTPException
from pathlib import Path
import re
import sys
import tempfile
import time
from urllib.request import urlopen


def file_hash(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def download_verified(url, destination, sha256, attempts=4):
    destination = Path(destination)
    if not re.fullmatch(r'[0-9a-fA-F]{64}', sha256) or attempts < 1:
        raise ValueError('A SHA256 checksum and at least one attempt are required')
    sha256 = sha256.lower()
    if destination.is_file() and file_hash(destination) == sha256:
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    for attempt in range(1, attempts + 1):
        partial = None
        try:
            with tempfile.NamedTemporaryFile(
                dir=destination.parent, prefix=f'.{destination.name}.',
                suffix='.part', delete=False,
            ) as output:
                partial = Path(output.name)
                digest = hashlib.sha256()
                with urlopen(url, timeout=60) as response:
                    while chunk := response.read(1024 * 1024):
                        output.write(chunk)
                        digest.update(chunk)
            if digest.hexdigest() != sha256:
                raise ValueError('Downloaded dependency checksum mismatch')
            partial.replace(destination)
            return
        except (OSError, ValueError, HTTPException) as error:
            print(f'{destination.name}: download attempt {attempt}/{attempts} '
                  f'failed ({type(error).__name__})', file=sys.stderr)
            if attempt == attempts:
                raise RuntimeError(
                    f'Unable to download verified {destination.name} after {attempts} attempts'
                ) from error
        finally:
            if partial is not None:
                partial.unlink(missing_ok=True)
        time.sleep(min(5 * 2 ** (attempt - 1), 30))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('url')
    parser.add_argument('destination', type=Path)
    parser.add_argument('sha256')
    args = parser.parse_args()
    download_verified(args.url, args.destination, args.sha256)
