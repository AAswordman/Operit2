"""Actual Python stdlib and dynamically loaded extension acceptance workload."""
import hashlib
import json
import math
import pathlib
import sqlite3
import tempfile

result = {
    'sqrt': math.sqrt(2.25),
    'sha256': hashlib.sha256(b'operit').hexdigest(),
    'sqlite': sqlite3.connect(':memory:').execute('select 6*7').fetchone()[0],
    'sum': sum(i*i for i in range(1000)),
}
assert result == {
    'sqrt': 1.5,
    'sha256': '399b85ed6523433a9edaa8f5083982ec8ac04e880d4f5741a7776833552ddf49',
    'sqlite': 42,
    'sum': 332833500,
}
print(json.dumps({'test': 'python-environment', 'ok': True, **result}, sort_keys=True))
