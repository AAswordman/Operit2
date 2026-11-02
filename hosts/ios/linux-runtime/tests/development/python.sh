set -eu
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
python3 -m venv "$work/venv"
"$work/venv/bin/python" -c "import sys,sqlite3; assert sys.prefix != sys.base_prefix; assert sqlite3.connect(':memory:').execute('select 42').fetchone()[0] == 42"
"$work/venv/bin/python" -m pip --version
printf 'PYTHON_VENV_PIP_OK\n'
