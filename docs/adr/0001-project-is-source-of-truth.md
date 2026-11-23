# The Project is the source of truth; the Cart is an export target
_Partly superseded by [ADR 0024](0024-the-song-has-4-tracks-one-per-channel.md): the Song has 4 Tracks, one per channel, and Export no longer picks them._

The tracker keeps its own Project format, which is richer than a PICO-8 Cart: it has 8 Tracks, Chain transposes and an unbounded Song. Export compiles a Project into a Cart's SFX and Patterns and may lose information, and Import reconstructs a Project from any Cart on a best-effort basis. We chose this over treating the Cart as the only source of truth because the M8 workflow doesn't fit inside PICO-8's 64 SFX and 64 Patterns. Making every edit obey those limits would cripple the workflow.

## Consequences

- Export picks 4 of the 8 Tracks.
- Export writes only the SFX and Pattern slot ranges the Project claims, and leaves the rest of the Cart (other SFX, code, graphics, map) untouched.
- Import → Export is not guaranteed to round-trip byte for byte.
