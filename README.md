# vitals

A terminal monitor of CPU, memory, swap and disk space that fits any window:
htop's header without the process list, plus how much of their usage limits
Claude Code and Codex have used. CPU is the share of all cores together, so
8 of 10 cores fully busy is 80%, never 800%. 50 color themes; resize the pane
and the layout follows; every setting is saved as you go.

```text
 ● studio · macOS 26.0 · Apple M4 Pro · 12 cores (8P+4E)                                    up 3d 5h · every 1s
╭ CPU ────────────────────────────────────────────────── 40% ╮╭ Memory ─────────────────────────────────── 64% ╮
│████████████████████████▎░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░││████████████████████████████████████████▌░░░░░░░│
│■ user 27% · ■ system 12% · ■ nice 1% · idle 60% · 52°C     ││15.2G of 24.0G · pressure normal                │
│load 4.21 3.87 3.02 · 35% of 12 cores                       ││■ app 10.0G · ■ wired 3.2G · ■ compressed 2.0G  │
│612 processes · 3148 threads                                ││■ cached 5.0G                                   │
│                                                            ││                                                │
│                                                            ││                                                │
│                                                            ││                                                │
│                                                            ││                                                │
│                                                            ││                                  ⣀⣀⣀⡀       ⢀⣀⣀│
│                                                            ││⣤⣤⣶⣶⣶⣶⣦⣤⣤⣤⣤⣤⣤⣤⣶⣶⣶⣶⣾⣿⣿⣿⣶⣶⣶⣶⣶⣶⣶⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│                                                            ││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│        ⣀⣀                                               ⣀⣤⣤││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│    ⣀⣤⣾⣿⣿⣿⣷⣆                                          ⢀⣤⣾⣿⣿⣿││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│⣿⣷⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡀                                      ⣠⣴⣿⣿⣿⣿⣿⣿││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡄                                ⣀⣠⣤⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡄                            ⣠⣶⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿││⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣆        ⣠⣶⣿⣿⣷⣦⡀         ⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│╰────────────────────────────────────────────────╯
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⡀   ⢀⣴⣾⣿⣿⣿⣿⣿⣿⣿⣷⣄     ⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│╭ Swap ───────────────────────────────────── 27% ╮
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣶⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣤⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿││████████████▊░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿││819M of 3.0G · in 0B/s · out 48.0K/s            │
│ 0 █████████▍  94%   4 ███▊░░░░░░  37%   8 ███▎░░░░░░  33%  ││                                                │
│ 1 ████████▊░  88%   5 ██▉░░░░░░░  29%   9 ██▌░░░░░░░  25%  ││                                                │
│ 2 ███████▏░░  71%   6 ██▏░░░░░░░  21%  10 █▎░░░░░░░░  12%  ││                                                │
│ 3 █████▎░░░░  52%   7 █▍░░░░░░░░  14%  11 ▊░░░░░░░░░   8%  ││                                                │
╰────────────────────────────────────────────────────────────╯│⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣄⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀│
╭ AI limits ─────────────────────────────────────────────────╮│⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿│
│Claude Max 5x · live                                        │╰────────────────────────────────────────────────╯
│  session █████████▏░░░░░░  57%  resets 18:51 · in 1h 32m   │╭ Disks ─────────────────────────────────────────╮
│  week    █████░░░░░░░░░░░  31%  resets Tue 20:19 · in 4d 3h││/               ███████████░░░░░  69%  291G free│
│Codex Plus · 25m ago                                        ││  Macintosh HD · 635G of 926G · read 1.2M/s     │
│  session █▉░░░░░░░░░░░░░░  12%  resets 20:39 · in 3h 20m   ││/Volumes/Backup ██████████████▌░  91%  169G free│
│  week    █████████████▊░░  86%  resets Sun 23:19 · in 2d 6h││  Backup · 1.7T of 1.8T · read 0B/s · write 0B/s│
╰────────────────────────────────────────────────────────────╯╰────────────────────────────────────────────────╯
 t theme  v graph style  g graphs  c cores  l AI limits  +- interval  ? help  q quit
```

<details>
<summary>A small pane, and a two-line strip</summary>

```text
 ● studio · macOS 26.0                up 3d 5h · every 1s
CPU        40% █████████████████▍░░░░░░░░░░░░░░░░░░░░░░░░░
■ user 27% · ■ system 12% · ■ nice 1% · idle 60% · 52°C
load 4.21 3.87 3.02 · 35% of 12 cores
612 processes · 3148 threads
Mem        64% ████████████████████████████████████▎░░░░░░
15.2G of 24.0G · pressure normal
Swap       27% ███████████▌░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
819M of 3.0G · in 0B/s · out 48.0K/s
/          69% █████████████████████████████▌░░░░░░░░░░░░░
Backup     91% ███████████████████████████████████████▏░░░
Claude 5h  57% ████████████████████████▌░░░░░░░░░░░░░░░░░░
Claude 7d  31% █████████████▍░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
Codex 5h   12% █████▏░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
Codex 7d   86% █████████████████████████████████████░░░░░░
 t theme  v graph style  g graphs  c cores  l AI limits
```

```text
CPU  40% █████████▋░░░░░░░░░░░░░░  Mem  64% ██████████████▍░░  Swap  27% ████▎░░░░░░░░░░░  /       69% █████████▋░░░░  Claude 5h  57% █████████▏░░░░░░
load 4.21 3.87 3.02                15.2G of 24.0G              819M of 3.0G · in 0B/s      Backup  91% ████████████▊░  Claude 7d  31% █████░░░░░░░░░░░
```

</details>

On a real terminal the bars and graphs are in the colors of the theme you pick:
user time green, system time red and nice time blue like htop, and every
percentage from green to yellow to red as it fills up.

## Features

- **CPU as a share of the whole computer.** All cores together, the way
  Activity Monitor and btop count it, split into user, system and nice time
  (plus iowait and steal on Linux) with htop's colors.
- **htop's header.** Load average over 1, 5 and 15 minutes, also as a share of
  the cores; processes and threads (and running threads on Linux); uptime;
  the CPU temperature where a sensor reports it; performance and efficiency
  cores on Apple silicon.
- **Each core.** A bar per core, numbered down the columns like htop, or a
  strip with a column per core when space is short. `c` hides them.
- **Memory the way your system counts it.** On macOS app, wired and compressed
  memory, cached files and memory pressure, like Activity Monitor. On Linux
  used, shared, buffers and cache like htop, available memory and pressure
  stall.
- **Swap**, and how fast pages move in and out of it.
- **Disk space** of every disk, with read and write speed. The volumes of an
  APFS container share their space, so they show once.
- **AI limits.** The 5-hour session and the week of Claude Code and Codex:
  how much is used and when each starts over, as a time and a countdown.
- **Graphs** of CPU, memory and swap over the last minutes, in braille dots or
  blocks (`v`).
- **Responsive.** Every way of placing the panels in columns is tried and the
  one that shows the most wins: graphs and core bars in a full screen, bare
  lines in a small tiling pane, one bar per column in a single-line strip.
  Nothing is ever cut in half.
- **50 themes.** `t` lists color themes after well-known editor themes: Tokyo
  Night, Dracula, Catppuccin, Gruvbox, Nord, Solarized and more. Moving
  through the list shows each one at once; type to filter.
- **Light.** Under 1% of one core. Disk space and sensors, which cost more to
  read, are read every 10 seconds; the rest at the interval you pick.
- **Saved as you go**, and **in English or Portuguese**, picked from `$LANG`
  (or `--lang`).

## Install

Download the archive for your system from the [latest
release](https://github.com/victorlcampos/vitals/releases/latest): macOS
(Apple silicon and Intel), Linux (x86_64 and ARM) or Windows. Unpack it and
put `vitals` somewhere on your `PATH`, such as `~/.local/bin`. On macOS, a
file a browser downloaded stays quarantined until you allow it: run `xattr -d
com.apple.quarantine vitals` once.

Or build it, with Rust 1.88 or newer ([rustup.rs](https://rustup.rs)):

```sh
cargo install --git https://github.com/victorlcampos/vitals
```

Runs on macOS and Linux; on Windows it shows the totals, without the split of
CPU time.

### Update

```sh
vitals update           # installs the latest release over this vitals
vitals update --check   # only tells whether there is a newer one
```

The new version replaces the old one only after it matches the SHA-256
published with the release and reports the right version.

## Usage

```sh
vitals                          # everything
vitals --theme "tokyo night"    # see --list-themes
vitals --interval 2             # read every 2 seconds (0.25 to 30)
vitals --graph blocks --no-cores
vitals --no-limits              # no AI limits, and no network at all
vitals --claude-dir ~/.claude-2  # only this Claude account
```

Command-line options work as if you did the same in the app, so they are
saved too. Run `vitals --help` for every option.

### Keys

| Key | Action |
| --- | --- |
| `t` | Color theme |
| `v` | Graph style: braille or blocks (brings hidden graphs back first) |
| `g` | Show or hide the graphs |
| `c` | Show or hide each core |
| `l` | Show or hide the AI limits |
| `+` `-` | Read less or more often, from 0.25 to 30 seconds |
| `?` | Help |
| `q`, `Esc` or `Ctrl+C` | Quit |

In the theme list: type to filter, `↑`/`↓` to try, `Enter` to keep, `Esc` to
go back to the theme you had.

### Themes

`Terminal` (the default) keeps your terminal's own colors. The other 50 are
the same as in [meridian](https://github.com/victorlcampos/meridian): Tokyo
Night, Dracula, One Dark, Monokai, Nord, Gruvbox, Solarized, Catppuccin, Rosé
Pine, Kanagawa, Everforest, Ayu, GitHub, VS Code, Material, Night Owl and more.
Terminals that do not advertise 24-bit color (`COLORTERM=truecolor`) get the
nearest colors of the 256-color palette.

## What the numbers mean

**CPU.** The time every core spent since the last reading, added up: 8 of 10
cores fully busy and 2 idle is 80%. Busy is user, nice, system (interrupts
included) and steal time; iowait is idle time spent waiting for the disk, so
it does not count as busy. Each core's bar is its own share.

**Load average.** Threads running or waiting to run (on Linux, also those
waiting for the disk), averaged over 1, 5 and 15 minutes, as htop shows it.
The share of the cores next to it is the 1-minute load over the core count: a
load of 8 on 10 cores is 80%, and above 100% work is waiting for a core.

**Memory.** On macOS used memory is app, wired and compressed memory, Activity
Monitor's *Memory Used*; cached files are given back when programs need them.
Memory pressure is what Activity Monitor graphs. On Linux used memory is what
htop counts, total minus free, buffers and cache, with shared memory (tmpfs)
as used; available is the kernel's estimate of what programs can still get.

**Disks.** Free space is what programs can use; on macOS it includes purgeable
space, like Finder. Sizes are in binary units like htop and `df -h`: 1G is
1024³ bytes.

## AI limits

vitals shows the 5-hour session window and the weekly window of each tool, how
much of each is used and when it starts over.

- **Claude Code.** Every 3 minutes vitals asks `api.anthropic.com/api/oauth/usage`,
  the address Claude Code's `/usage` asks, with the login Claude Code keeps: in
  the macOS keychain, read with the same `security` command Claude Code uses,
  or in `~/.claude/.credentials.json`. vitals never renews that login, so it
  cannot log Claude Code out: when the login has expired, it waits for Claude
  Code to renew it and meanwhile shows the last figures Claude Code saved in
  `~/.claude.json`, with how old they are. Weekly limits of a single model,
  such as Fable's, show up even at 0%. Other folders beside `~/.claude`
  with a Claude Code login, such as `~/.claude-2` for a second account,
  show too, each named after its folder (Claude 2); `--claude-dir` picks
  the folders instead.
- **Codex.** vitals asks `chatgpt.com/backend-api/wham/usage`, the address
  Codex's `/status` asks, with the login in `~/.codex/auth.json`; otherwise it
  reads the figures of the last Codex session in `~/.codex/sessions`. Plans
  without a 5-hour window show only the week.
- **OpenCode.** OpenCode Go has 5-hour, weekly and monthly limits, but they
  only show in the web console at opencode.ai: OpenCode keeps no copy on disk
  and offers no address to ask, so vitals cannot show them.

The logins are only read to ask those two addresses; vitals keeps no copy and
sends them nowhere else. `l` hides the panel and stops the asking;
`--no-limits` starts without it.

## Saved state

Theme, graph style, which parts show and the interval live in
`~/.config/vitals/state.json` (or `$XDG_CONFIG_HOME/vitals/state.json`),
rewritten on every change, all at once, so a crash leaves either the old file
or the new one. A file that cannot be read is kept as `state.json.broken`
rather than lost.

## Tiling window managers

The layout recalculates on every resize: a full screen, half of one, a narrow
column, a short strip or a single line all get their own arrangement. Copies
running side by side share the state file, so a theme picked in one shows up
in the others. For panes that should stay apart, give each its own file:

```sh
# i3 / sway
exec alacritty --class vitals -e vitals --state ~/.config/vitals/bar.json
```

## Em português

Monitor de terminal de CPU, memória, swap e espaço em disco que cabe em
qualquer janela: o cabeçalho do htop sem a lista de processos, mais os limites
de uso do Claude Code e do Codex. A CPU é a parte de todos os núcleos juntos:
8 de 10 núcleos ocupados são 80%, nunca 800%. Mostra o tempo de CPU dividido
como no htop (usuário, sistema, nice; iowait e steal no Linux), a carga média
de 1, 5 e 15 minutos também como parte dos núcleos, processos, threads, tempo
ligado, temperatura e cada núcleo; a memória como o Monitor de Atividade conta
no macOS (apps, fixa, comprimida, cache e pressão) ou como o htop no Linux; o
swap e a velocidade de entrada e saída; o espaço livre de cada disco, com
leitura e escrita. O painel *Limites de IA* mostra a janela de 5 horas (sessão)
e a semanal do Claude Code e do Codex, quanto foi usado e quando cada uma
renova, com a hora e quanto falta; os dados vêm dos mesmos endereços que o
`/usage` do Claude e o `/status` do Codex consultam, com o login que eles
guardam, que o vitals só lê e nunca renova. Os limites do OpenCode Go só
aparecem no console web do opencode.ai. `t` troca o tema entre 50 temas de
editores, `v` o estilo do gráfico, `g` mostra ou esconde os gráficos, `c` os
núcleos, `l` os limites de IA, `+` e `-` mudam o intervalo. O layout se
refaz a cada redimensionamento, de tela cheia a uma faixa de uma linha. Tudo
é salvo a cada mudança em `~/.config/vitals/state.json`. A interface fica em
português quando `$LANG` começa com `pt`. `vitals update` instala a versão
mais nova.

## License

[MIT](LICENSE)
