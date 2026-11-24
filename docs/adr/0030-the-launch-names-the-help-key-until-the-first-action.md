# The launch names the Help key until the first action

Every launch shows a fading info notice that names the Help key, such as `^H HELP`, read from the active keymap like every other hint (ADR 0028). It is the one hint that goes on the status line without a question, list or dialog. A new user has no other way to learn the Help key, because the idle status line names no keys.

The notice comes last in the launch. A settings report stays on the status line until the user dismisses it with Cancel, and then the notice shows. When the launch reopens the last Project, the notice shows once that file is read, so it replaces `OPENED …`, which still goes in the Log. The idle status line names the open Project anyway.

The notice ends at the user's first action, apart from the Cancel that dismisses a settings report. An action takes the notice off the status line, so a list opened straight after launch shows its keys. Once the user has acted, a notice that hasn't shown yet is dropped, so it never turns up mid-session over a message the user asked for.

The notice is a key hint, not a result, so it stays out of the Log. This narrows ADR 0010, which puts every message in the Log.

## Consequences

- A user who acts before dismissing a settings report doesn't see the notice that launch.
- The notice is never stored in `state.ron`. It shows on every launch.
