use super::{Chain, ChainId, CustomInstrument, Phrase, Project, SongRowMarks, TRACKS, TargetCart};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Swap<T> {
    before: T,
    after: T,
}

impl<T: Clone + PartialEq> Swap<T> {
    fn between(before: &T, after: &T) -> Option<Self> {
        (before != after).then(|| Self {
            before: before.clone(),
            after: after.clone(),
        })
    }

    fn value(&self, way: Way) -> &T {
        match way {
            Way::Back => &self.before,
            Way::Forward => &self.after,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Way {
    Back,
    Forward,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectChange {
    phrases: Vec<(usize, Swap<Phrase>)>,
    chains: Vec<(usize, Swap<Chain>)>,
    song_rows: Vec<(usize, Swap<[Option<ChainId>; TRACKS]>)>,
    song_marks: Vec<(usize, Swap<SongRowMarks>)>,
    default_speed: Option<Swap<u8>>,
    target_cart: Option<Swap<Option<TargetCart>>>,
    custom_instruments: Vec<(u8, Swap<Option<CustomInstrument>>)>,
}

fn changed_items<T: Clone + PartialEq>(before: &[T], after: &[T]) -> Vec<(usize, Swap<T>)> {
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter_map(|(index, (before, after))| Some((index, Swap::between(before, after)?)))
        .collect()
}

fn without_claimed_bytes_hash(target_cart: Option<&TargetCart>) -> Option<TargetCart> {
    target_cart.cloned().map(|mut target_cart| {
        target_cart.claimed_bytes_hash = None;
        target_cart
    })
}

impl ProjectChange {
    pub(crate) fn between(before: &Project, after: &Project) -> Self {
        let target_cart = Swap::between(
            &without_claimed_bytes_hash(before.target_cart.as_ref()),
            &without_claimed_bytes_hash(after.target_cart.as_ref()),
        )
        .map(|_| Swap {
            before: before.target_cart.clone(),
            after: after.target_cart.clone(),
        });
        let instrument_ids = before
            .custom_instruments
            .keys()
            .chain(after.custom_instruments.keys());
        let mut custom_instruments: Vec<_> = instrument_ids
            .filter_map(|id| {
                let swap = Swap::between(
                    &before.custom_instruments.get(id).cloned(),
                    &after.custom_instruments.get(id).cloned(),
                )?;
                Some((*id, swap))
            })
            .collect();
        custom_instruments.dedup_by_key(|(id, _)| *id);
        Self {
            phrases: changed_items(&before.phrases, &after.phrases),
            chains: changed_items(&before.chains, &after.chains),
            song_rows: changed_items(&before.song, &after.song),
            song_marks: changed_items(&before.song_marks, &after.song_marks),
            default_speed: Swap::between(&before.default_speed, &after.default_speed),
            target_cart,
            custom_instruments,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub(crate) fn undo(&self, project: &mut Project) {
        self.apply(project, Way::Back);
    }

    pub(crate) fn redo(&self, project: &mut Project) {
        self.apply(project, Way::Forward);
    }

    fn apply(&self, project: &mut Project, way: Way) {
        for (index, swap) in &self.phrases {
            project.phrases[*index] = *swap.value(way);
        }
        for (index, swap) in &self.chains {
            project.chains[*index] = *swap.value(way);
        }
        for (index, swap) in &self.song_rows {
            project.song[*index] = *swap.value(way);
        }
        for (index, swap) in &self.song_marks {
            project.song_marks[*index] = *swap.value(way);
        }
        if let Some(swap) = &self.default_speed {
            project.default_speed = *swap.value(way);
        }
        if let Some(swap) = &self.target_cart {
            project.target_cart =
                keeping_current_claimed_bytes_hash(swap.value(way), project.target_cart.as_ref());
        }
        for (id, swap) in &self.custom_instruments {
            project.set_custom_instrument(*id, swap.value(way).clone());
        }
    }
}

fn keeping_current_claimed_bytes_hash(
    restored: &Option<TargetCart>,
    current: Option<&TargetCart>,
) -> Option<TargetCart> {
    let mut restored = restored.clone();
    if let (Some(restored), Some(current)) = (restored.as_mut(), current)
        && restored.path == current.path
    {
        restored.claimed_bytes_hash = current.claimed_bytes_hash.clone();
    }
    restored
}
