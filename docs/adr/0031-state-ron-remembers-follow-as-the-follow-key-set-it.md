# state.ron remembers Follow as the Follow key set it

Moving the cursor by hand turns Follow off at once, as before, but only for this session. `state.ron` keeps the value the user last set with the Follow key. We used to save whatever Follow was after each action. Because the cursor moves by hand all the time, that turned the remembered Follow off almost every session, so it stayed on across launches only if the user never moved the cursor after turning it on.

## Consequences

- At launch, Follow starts as the Follow key last set it, even if a hand move turned it off before the last quit.
- A hand move on its own never writes `state.ron`.
- After a hand move, pressing the Follow key turns Follow back on, and `state.ron` keeps on. If on was already the remembered value, nothing is written.
