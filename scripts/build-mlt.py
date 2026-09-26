#!/usr/bin/env python3
"""Build the pinned minimal MLT SDK using existing FFmpeg/SDL2 development libraries."""
from pathlib import Path
import hashlib,subprocess,tarfile,re,sys,json
root=Path(__file__).resolve().parents[1]/'desktop/native'
source=json.loads((Path(__file__).parent/'mlt-source.json').read_text())
version=source['revision']
archive=root/f'vendor/mlt-{version}.tar.gz';archive.parent.mkdir(parents=True,exist_ok=True)
if not archive.exists():subprocess.run(['curl','-fL',f'https://codeload.github.com/mltframework/mlt/tar.gz/{version}','-o',str(archive)],check=True)
if hashlib.sha256(archive.read_bytes()).hexdigest()!=source['sha256']:
    sys.exit('MLT archive checksum mismatch')
src=root/f'vendor/mlt-{version}'
if not src.exists():
    if not hasattr(tarfile, 'data_filter'): sys.exit('MLT SDK build requires Python with tarfile.data_filter (Python 3.12+ recommended).')
    with tarfile.open(archive) as tar:tar.extractall(archive.parent,filter='data')
modules=re.findall(r'option\((MOD_\w+)',(src/'CMakeLists.txt').read_text())
args=['cmake','-S',str(src),'-B',str(root/'build-mlt'/version),'-DCMAKE_BUILD_TYPE=Release','-DCMAKE_INSTALL_PREFIX='+str(root/'runtime'),'-DBUILD_TESTING=OFF','-DBUILD_DOCS=OFF','-DUSE_AVDEVICE=OFF','-DGPL=OFF','-DGPL3=OFF']
args+=['-D'+m+'='+('ON' if m in ['MOD_AVFORMAT','MOD_PLUS','MOD_RESAMPLE','MOD_SDL2','MOD_XML'] else 'OFF') for m in modules]
subprocess.run(args,check=True)
subprocess.run(['cmake','--build',str(root/'build-mlt'/version),'--config','Release','-j','4'],check=True)
subprocess.run(['cmake','--install',str(root/'build-mlt'/version),'--config','Release'],check=True)
