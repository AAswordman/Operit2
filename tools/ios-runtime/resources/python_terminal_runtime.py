import code
import contextlib
import io
import traceback

_sessions = {}


class _TerminalConsole(code.InteractiveConsole):
    """Captures interpreter diagnostics into the active terminal output stream."""

    def __init__(self, namespace, stream):
        super().__init__(locals=namespace)
        self._stream = stream

    def write(self, data):
        """Writes one Python interpreter diagnostic to the terminal output stream."""
        self._stream.write(data)


def operit_create_terminal(session_id):
    """Creates one persistent interactive CPython console namespace."""
    stream = io.StringIO()
    namespace = {"__name__": "__main__", "__package__": None}
    _sessions[session_id] = {
        "console": _TerminalConsole(namespace, stream),
        "input": "",
        "output": ["Python embedded in Operit iOS\n>>> "],
        "screen": "Python embedded in Operit iOS\n>>> ",
        "stream": stream,
    }


def operit_write_terminal(session_id, text):
    """Executes every complete input line and returns terminal output emitted by CPython."""
    session = _sessions[session_id]
    source = session["input"] + text.replace("\r\n", "\n").replace("\r", "\n")
    lines = source.split("\n")
    session["input"] = lines.pop()
    emitted = []
    for line in lines:
        prompt = "... " if session["console"].buffer else ">>> "
        session["screen"] += prompt + line + "\n"
        session["stream"].seek(0)
        session["stream"].truncate(0)
        try:
            with contextlib.redirect_stdout(session["stream"]), contextlib.redirect_stderr(session["stream"]):
                more = session["console"].push(line)
        except BaseException:
            traceback.print_exc(file=session["stream"])
            more = False
        output = session["stream"].getvalue()
        emitted.append(output)
        session["screen"] += output
        next_prompt = "... " if more else ">>> "
        emitted.append(next_prompt)
        session["screen"] += next_prompt
    result = "".join(emitted)
    session["output"].append(result)
    return result


def operit_read_terminal(session_id):
    """Drains accumulated CPython terminal output for one session."""
    session = _sessions[session_id]
    result = "".join(session["output"])
    session["output"].clear()
    return result


def operit_screen_terminal(session_id):
    """Returns the retained terminal transcript for one CPython session."""
    return _sessions[session_id]["screen"]


def operit_close_terminal(session_id):
    """Releases one CPython console namespace."""
    del _sessions[session_id]
