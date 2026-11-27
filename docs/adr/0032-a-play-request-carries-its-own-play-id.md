# A play request carries its own PlayId

The Transport gives every play it starts a `PlayId` and puts it in the `AudioRequest`. The Player never numbers plays itself: it copies the id it was given into the `finished_play` of its `PlaybackReport`. The Transport then knows the latest play has ended only when a report names that play's id. We used to keep two counters, one in the App core and one in the Player, each adding 1 for every play request. They agreed only because every play went through the same App function. Any play request sent another way would have shifted one counter against the other, and from then on "finished" reports would have been matched to the wrong play. We chose to carry the id with the request so that no two counters have to agree.

## Consequences

- Only the requests that start a play carry a `PlayId`: song, chain, chain on its own, phrase and instrument audition. A step preview, Stop, a resent Project and a Mute or Solo change carry none.
- `finished_play` is `None` until the Player has finished a play.
- A report from an older play never ends the latest one, however late it arrives.
- The Window shell passes reports from the audio thread to the App core without reading them, so only the type changes there.
