#!/usr/bin/env bash
# Ensure UTF-8 locale so emoji bytes are passed through correctly
export LANG="${LANG:-en_US.UTF-8}"
export LC_ALL="${LC_ALL:-en_US.UTF-8}"

input=$(cat)

# ── Data extraction ──────────────────────────────────────────────────────────
cwd=$(echo "$input" | jq -r '.cwd // .workspace.current_dir // empty')
model=$(echo "$input" | jq -r '.model.display_name // empty')
output_style=$(echo "$input" | jq -r '.output_style.name // empty')
total_input_tokens=$(echo "$input" | jq -r '.context_window.total_input_tokens // empty')
ctx_window_size=$(echo "$input"    | jq -r '.context_window.context_window_size // empty')
used_pct=$(echo "$input"           | jq -r '.context_window.used_percentage // empty')
thinking=$(echo "$input"           | jq -r '.thinking.enabled // false')
effort=$(echo "$input"             | jq -r '.effort.level // empty')
vim_mode=$(echo "$input"           | jq -r '.vim.mode // empty')

# Shorten $HOME to ~
home_dir="$HOME"
display_cwd="${cwd/#$home_dir/\~}"

# Git branch (skip optional locks to avoid blocking)
branch=""
if [ -n "$cwd" ]; then
    branch=$(git --no-optional-locks -C "$cwd" symbolic-ref --short HEAD 2>/dev/null \
             || git --no-optional-locks -C "$cwd" rev-parse --short HEAD 2>/dev/null)
fi

# ── ANSI palette using $'...' so escapes are real bytes at assignment time ────
# Accent 1: soft sky blue    Accent 2: warm amber    Accent 3: muted violet
blue=$'\033[38;5;75m'       # sky blue  — model, cwd
green=$'\033[38;5;114m'     # sage green — branch
amber=$'\033[38;5;179m'     # warm amber — tokens
violet=$'\033[38;5;141m'    # muted violet — context bar / pct
dim=$'\033[38;5;242m'       # dim grey — separators
cyan=$'\033[38;5;80m'       # teal — extras (style, effort, vim)
red=$'\033[38;5;203m'       # soft red — high context warning (>80%)
bold=$'\033[1m'
reset=$'\033[0m'

sep="${dim} │ ${reset}"

# ── Emoji literals (UTF-8 characters embedded directly) ──────────────────────
e_model='🤖'
e_dir='📁'
e_branch='🌿'
e_token='🪙'
e_ctx='📊'
e_style='🎨'
e_think='🧠'
e_vim='⌨'

# ── Context bar (10 blocks, proportional to used_percentage) ─────────────────
ctx_bar=""
if [ -n "$used_pct" ]; then
    pct_int=$(printf '%.0f' "$used_pct")
    filled=$(( pct_int / 10 ))
    empty=$(( 10 - filled ))
    bar=""
    for i in $(seq 1 "$filled"); do bar="${bar}▰"; done
    for i in $(seq 1 "$empty");  do bar="${bar}▱"; done
    if [ "$pct_int" -ge 80 ]; then
        ctx_bar="${red}${bar} ${pct_int}%${reset}"
    else
        ctx_bar="${violet}${bar} ${pct_int}%${reset}"
    fi
fi

# ── Token count (formatted with commas via awk) ───────────────────────────────
tok_segment=""
if [ -n "$total_input_tokens" ] && [ -n "$ctx_window_size" ]; then
    tokens_fmt=$(printf '%d' "$total_input_tokens" \
                 | awk '{printf "%'"'"'d", $1}' 2>/dev/null \
                 || printf '%d' "$total_input_tokens")
    ctx_fmt=$(printf '%d' "$ctx_window_size" \
              | awk '{printf "%'"'"'d", $1}' 2>/dev/null \
              || printf '%d' "$ctx_window_size")
    tok_segment="${amber}${e_token} ${tokens_fmt}${dim}/${reset}${amber}${ctx_fmt}${reset}"
fi

# ── Assemble output ───────────────────────────────────────────────────────────

# Model
if [ -n "$model" ]; then
    printf '%s' "${blue}${e_model} ${bold}${model}${reset}"
fi

# cwd
if [ -n "$display_cwd" ]; then
    printf '%s' "${sep}${blue}${e_dir} ${display_cwd}${reset}"
fi

# branch
if [ -n "$branch" ]; then
    printf '%s' "${sep}${green}${e_branch} ${branch}${reset}"
fi

# token count
if [ -n "$tok_segment" ]; then
    printf '%s' "${sep}${tok_segment}"
fi

# context bar
if [ -n "$ctx_bar" ]; then
    printf '%s' "${sep}${e_ctx} ${ctx_bar}"
fi

# output style (omit when "default" or empty)
if [ -n "$output_style" ] && [ "$output_style" != "default" ] && [ "$output_style" != "Default" ]; then
    printf '%s' "${sep}${cyan}${e_style} ${output_style}${reset}"
fi

# extended thinking indicator
if [ "$thinking" = "true" ]; then
    if [ -n "$effort" ]; then
        printf '%s' "${sep}${cyan}${e_think} ${effort}${reset}"
    else
        printf '%s' "${sep}${cyan}${e_think}${reset}"
    fi
fi

# vim mode (only when vim mode is active)
if [ -n "$vim_mode" ]; then
    printf '%s' "${sep}${cyan}${e_vim} ${vim_mode}${reset}"
fi
