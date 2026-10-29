#!/usr/bin/env python3
"""Build the pinned USearch C API plus the small memory ABI adapter using Emscripten."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

REVISION = 'a985f77553d2d58d5ef1516eafaae81480bb3c30'
ROOT = Path(__file__).resolve().parents[6]
HERE = Path(__file__).resolve().parent
EXPORTS = ['malloc', 'free', 'index_error', 'index_create', 'index_add_batch', 'index_search', 'index_search_filtered', 'index_score_keys', 'index_size', 'index_remove', 'index_save_size', 'index_save', 'index_load', 'index_destroy']

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, help='Existing checkout of the pinned USearch revision')
    args = parser.parse_args()
    source = args.source or Path(tempfile.gettempdir()) / 'operit-usearch-build' / ('USearch-' + REVISION)
    if not source.exists():
        source.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='operit-usearch-') as temporary:
            archive = Path(temporary) / 'source.tar.gz'
            urllib.request.urlretrieve('https://codeload.github.com/unum-cloud/USearch/tar.gz/' + REVISION, archive)
            with tarfile.open(archive) as package:
                package.extractall(source.parent, filter='data')
    if (source / '.git').exists():
        actual = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
        if actual != REVISION:
            raise SystemExit('USearch source must be pinned to ' + REVISION)
    compiler = os.environ.get('EMXX') or shutil.which('em++') or str(ROOT / '.ci-tools/emsdk/upstream/emscripten/em++')
    output = HERE.parent.parent / 'resources/usearch.wasm'
    output.parent.mkdir(parents=True, exist_ok=True)
    import json
    subprocess.run([compiler, str(source / 'c/lib.cpp'), str(HERE / 'index.cpp'), '-I' + str(source / 'include'), '-I' + str(source / 'c'), '-DUSEARCH_USE_NUMKONG=0', '-DUSEARCH_USE_OPENMP=0', '-DNDEBUG', '-std=c++17', '-O3', '-flto', '-msimd128', '--no-entry', '-sSTANDALONE_WASM=1', '-sALLOW_MEMORY_GROWTH=1', '-sFILESYSTEM=0', '-sEXPORTED_FUNCTIONS=' + json.dumps(['_' + name for name in EXPORTS]), '-o', str(output)], check=True)
    targets = [ROOT / 'plugins/packages/examples/toolpkg_usearch_demo/modules/usearch.wasm']
    for target in targets:
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(output, target)
    print('Built', output.stat().st_size, 'bytes from', REVISION)

if __name__ == '__main__':
    main()
