# https://github.com/emmanueltouzery/projectpad2
# shell integration for ppcli: control-space to run
ppcli-run() {
    output=$(ppcli --shell-integration)
    # split by NUL https://stackoverflow.com/a/2269760/516188
    pieces=( ${(ps.\0.)output} )
    case "$pieces[1]" in
        R) # R == run
            cur_folder=$(pwd)
            if [[ ! -z $pieces[3] ]]; then
               cmd="cd $pieces[3] && $pieces[2]"
            else
               cmd="$pieces[2]"
            fi
            echo -e "\e[3m$cmd\e[0m" # print with italics because it wasn't really _typed_
            print -s "$cmd" # https://stackoverflow.com/a/2816792/516188
            # need the </dev/tty and the stty so that ssh shells work
            # https://stackoverflow.com/questions/57539180/why-is-interactive-command-breaking-when-using-zsh-widget-to-execute-it#comment101556821_57539863
            # i need the printf to avoid ~0 and ~1 around pasting https://unix.stackexchange.com/a/196574/36566
            eval "stty sane; printf '\e[?2004l'; $cmd" </dev/tty
            cd $cur_folder
            # accept-line: give me a prompt, and that takes into account the
            # new history i've added with print -s (zle reset-prompt doesn't do that)
            zle && zle accept-line
            ;;
        P) # P == print to the prompt
            zle -U "$pieces[2]"
            ;;
        C) # C == Copy to the clipboard
            # https://stackoverflow.com/questions/42655304/how-do-i-check-if-a-variable-is-set-in-zsh/42655305
            if [[ -v WAYLAND_DISPLAY ]]; then
                wl-copy "$pieces[2]"
            else
                echo "$pieces[2]" | xsel --clipboard
            fi
            ;;
    esac
    if [[ ! -z "$pieces[4]" ]]; then
        echo "\n\nppcli has detected a new version is available.\nIt's recommended to upgrade by running:\n ppcli --upgrade\n new version URL: $pieces[4]"
    fi
}
zle -N ppcli-run
bindkey '^ ' ppcli-run

