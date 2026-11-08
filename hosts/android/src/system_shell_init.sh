PS1='$(__operit_status=$?; printf "\033]133;OperitPrompt=%s:%s\007" "$(printf "%s" "$PWD" | base64 | tr -d "\n")" "$__operit_status")'; PS2=''
