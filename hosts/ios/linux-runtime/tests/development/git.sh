set -eu
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
git init -q
printf 'content\n' > sample.txt
git add sample.txt
git -c user.name=Runtime -c user.email=runtime@example.invalid commit -qm 'runtime probe'
test "$(git show HEAD:sample.txt)" = content
test "$(git log -1 --format=%s)" = 'runtime probe'
test -z "$(git status --porcelain)"
printf 'GIT_COMMIT_OK\n'
