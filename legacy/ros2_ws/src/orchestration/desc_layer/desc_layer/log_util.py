"""彩色日志工具。

用法:
  from .log_util import info, warn, error
  info("desc", "HTTP", "POST /api/v1/tasks → 201", trace="a1b2")
  info("exec", "PLAN", "3 segments: 1→2→3→42")
"""

import datetime

ANSI = {
    "reset": "\033[0m",
    "red": "\033[31m",
    "green": "\033[32m",
    "yellow": "\033[33m",
    "blue": "\033[34m",
    "magenta": "\033[35m",
    "cyan": "\033[36m",
    "white": "\033[37m",
}

MOD_COLORS = {
    "PLAN": "yellow",
    "DRIVE": "white",
    "TASK": "magenta",
    "ACT": "green",
    "HTTP": "cyan",
    "WS": "blue",
}


def _ts() -> str:
    return datetime.datetime.now().strftime("%H:%M:%S")


def _fmt(mod: str, tag: str, msg: str, **kv) -> str:
    c = MOD_COLORS.get(tag, "white")
    parts = [f"{k}={v}" for k, v in kv.items() if v]
    suffix = f"  ({', '.join(parts)})" if parts else ""
    return (f"{ANSI[c]}[{mod}:{tag}]{ANSI['reset']} {msg}"
            f"\033[90m{suffix}\033[0m")


def info(mod: str, tag: str, msg: str, **kv) -> None:
    print(f"\033[32mINFO\033[0m {_ts()} {_fmt(mod, tag, msg, **kv)}", flush=True)


def warn(mod: str, tag: str, msg: str, **kv) -> None:
    print(f"\033[33mWARN\033[0m {_ts()} {_fmt(mod, tag, msg, **kv)}", flush=True)


def error(mod: str, tag: str, msg: str, **kv) -> None:
    print(f"\033[31mERROR\033[0m {_ts()} {_fmt(mod, tag, msg, **kv)}", flush=True)
