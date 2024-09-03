## Setup cron task via systemd

- these files should be placed at `$HOME/.config/systemd/user/`

- and then you can start this with `systemctl --user enable clashtui.timer`

- in this example, timer will call `clashtui.service` 1min after boot complete, every 24h after that.

- you can refer [arch wiki](https://wiki.archlinux.org/title/Systemd/Timers) for more usage.

