import argparse
import json
import os
import re
import time
from datetime import datetime
from pathlib import Path

from fastapi import FastAPI, Query
from fastapi.responses import FileResponse, JSONResponse, StreamingResponse
from fastapi.staticfiles import StaticFiles


APP_DIR = Path(__file__).resolve().parent
STATIC_DIR = APP_DIR / "static"


def looks_like_repo_root(path: Path) -> bool:
    if not path.is_dir():
        return False
    for child in path.iterdir():
        if not child.is_dir():
            continue
        name = child.name
        if re.match(r"^\d{2}_BPF_PROG_TYPE_", name):
            return True
    return False


def resolve_repo_root(cli_value: str | None) -> Path:
    if cli_value:
        return Path(cli_value).resolve()

    # Try common case: .../26_EBPF-TUI/ebpf-tui/web
    candidate = APP_DIR.parent.parent.parent
    if looks_like_repo_root(candidate):
        return candidate

    # Try walking up from current working directory
    cwd = Path.cwd().resolve()
    for parent in [cwd] + list(cwd.parents):
        if looks_like_repo_root(parent):
            return parent

    return cwd


def classify_event(line: str) -> str:
    text = line.lower()
    if "trace |" in text:
        return "trace"
    if re.search(r"\b(unload|stopped|stop)\b", text):
        return "stop"
    if re.search(r"\b(build)\b", text):
        return "build"
    if re.search(r"\b(load)\b", text) and "unload" not in text:
        return "load"
    if re.search(r"\b(run|running)\b", text):
        return "run"
    if re.search(r"\b(fail|failed)\b", text):
        return "fail"
    if re.search(r"\b(verify)\b", text):
        return "verify"
    return "log"


def parse_line(line: str) -> dict:
    line = line.rstrip("\n")
    module = "system"
    message = line
    if " | " in line:
        module, message = line.split(" | ", 1)
        module = module.strip() or "system"
        message = message.strip()
    return {
        "ts": datetime.now().isoformat(timespec="seconds"),
        "module": module,
        "event_type": classify_event(line),
        "message": message,
        "raw": line,
    }


def tail_lines(path: Path, count: int) -> list[str]:
    if not path.exists():
        return []
    with path.open("r", encoding="utf-8", errors="replace") as f:
        return f.read().splitlines()[-count:]


def ensure_log_file(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_text("", encoding="utf-8")


def create_app(repo_root: Path, log_path: Path) -> FastAPI:
    app = FastAPI()

    app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")

    @app.get("/")
    def index() -> FileResponse:
        return FileResponse(STATIC_DIR / "index.html")

    @app.get("/api/history")
    def history(lines: int = Query(300, ge=1, le=5000)) -> JSONResponse:
        items = [parse_line(l) for l in tail_lines(log_path, lines)]
        return JSONResponse({"items": items})

    @app.get("/events")
    def events() -> StreamingResponse:
        def stream():
            ensure_log_file(log_path)
            with log_path.open("r", encoding="utf-8", errors="replace") as f:
                f.seek(0, os.SEEK_END)
                counter = 0
                while True:
                    line = f.readline()
                    if not line:
                        time.sleep(0.5)
                        yield ": keep-alive\n\n"
                        continue
                    counter += 1
                    payload = parse_line(line)
                    data = json.dumps(payload, ensure_ascii=True)
                    yield f"id: {counter}\n"
                    yield "event: log\n"
                    yield f"data: {data}\n\n"

        return StreamingResponse(stream(), media_type="text/event-stream")

    @app.get("/api/info")
    def info() -> JSONResponse:
        return JSONResponse({
            "repo_root": str(repo_root),
            "log_path": str(log_path),
        })

    return app


def main() -> None:
    parser = argparse.ArgumentParser(description="ebpf-tui web view")
    parser.add_argument("--repo-root", default=None)
    parser.add_argument("--log-path", default=None)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", default=8000, type=int)
    args = parser.parse_args()

    repo_root = resolve_repo_root(args.repo_root)
    log_path = Path(args.log_path).resolve() if args.log_path else repo_root / "artifacts" / "status_window.log"

    ensure_log_file(log_path)

    app = create_app(repo_root, log_path)

    import uvicorn

    uvicorn.run(app, host=args.host, port=args.port, log_level="info")


if __name__ == "__main__":
    main()
