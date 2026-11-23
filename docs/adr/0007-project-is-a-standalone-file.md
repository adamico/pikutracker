# A Project is a standalone file, not embedded in its Cart

A Project is saved as one standalone `.ptrk` file. It holds at most one link to its Target Cart, as a path relative to the Project file, and the Claim is stored with that link. We considered embedding the Project, or a link to it, in a `__meta:pikutracker__` section, so the Cart would carry its own source. We rejected that for four reasons. A Cart can hold several Projects with Claims that don't overlap. `.p8.png` Carts may not keep `__meta__` sections. A Project can exist with no Cart, for example after a MIDI Import. And, by ADR 0001, the Project is the source of truth, so it should not live inside its own Export target.

## Consequences

- The file is RON text with an integer format version. Older versions are migrated forward, and newer versions are refused.
- Moving the Cart breaks the link. Export then asks the user to locate the Cart, and keeps the Claim.
- A New Cart is written to disk as soon as its name is confirmed, and becomes the Target Cart only once that write succeeds. So a missing Target Cart still means only that it moved.
- The Project stores a hash of its claimed bytes, taken at Cart Import, when a New Cart is made, and after each Export. If the Cart's claimed bytes no longer match, because they were edited in PICO-8, Export warns before overwriting them.
- Embedding the Project in the Cart is out of scope for the MVP.
