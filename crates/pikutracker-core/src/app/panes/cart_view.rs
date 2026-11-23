use super::log::{TAG_COLUMN, TEXT_COLUMN, TEXT_WIDTH, tag, tag_style, wrapped};
use super::{EMPTY, HEADING, VALUE, draw_title, stepped};
use crate::app::Direction;
use crate::app::claim_text::{ClaimText, named_slots};
use crate::app::focus::Focus;
use crate::app::layout::Area;
use crate::app::messages::Severity;
use crate::cart::{Cart, SLOTS, Slots};
use crate::export::{ExportFailure, ExportNeeds};
use crate::project::{Claim, TargetCart, custom_instrument_of_slot};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const NO_TARGET_CART: &str = "NO TARGET CART";
const PATH_WIDTH: usize = 22;
const CART_ROW: usize = 1;
const SFX_HEADING_ROW: usize = 3;
const PATTERNS_HEADING_ROW: usize = 5;
const SFX_MAP_ROW: usize = 8;
const PATTERN_MAP_ROW: usize = 13;
const FIRST_CHECK_ROW: usize = 19;
const MAP_COLUMNS: usize = 16;
const MAP_COLUMN: usize = 2;

const POINTER: Style = Style::fg(Colour::Yellow);
const FREE: Style = EMPTY;
const USED: Style = Style::fg(Colour::LightGrey);
const CLAIMED: Style = Style::fg(Colour::Green);
const OVERWRITTEN: Style = Style::fg(Colour::Red);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartField {
    Cart,
    Sfx,
    Patterns,
}

const FIELDS: [CartField; 3] = [CartField::Cart, CartField::Sfx, CartField::Patterns];

#[derive(Debug)]
enum TargetCartContents {
    Unknown,
    Read(Box<Cart>),
    Unreadable(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotMapMark {
    Free,
    Used,
    Claimed,
    Overwritten,
}

impl SlotMapMark {
    const fn mark(self) -> (&'static str, Style) {
        match self {
            Self::Free => (".", FREE),
            Self::Used => ("O", USED),
            Self::Claimed => ("#", CLAIMED),
            Self::Overwritten => ("!", OVERWRITTEN),
        }
    }
}

#[derive(Debug)]
struct SlotUse {
    used: Slots,
    own: Slots,
}

impl SlotUse {
    fn of(cart: &Cart, hash: Option<&str>) -> Self {
        let every = 0..SLOTS as u8;
        Self {
            used: Slots {
                sfx: every
                    .clone()
                    .filter(|slot| cart.sfx(*slot).is_used())
                    .collect(),
                patterns: every
                    .filter(|slot| !cart.pattern(*slot).is_empty())
                    .collect(),
            },
            own: hash.map_or_else(Slots::default, |hash| cart.unchanged_since(hash)),
        }
    }

    fn free_or_own(used: &[u8], own: &[u8]) -> Vec<u8> {
        (0..SLOTS as u8)
            .filter(|slot| !used.contains(slot) || own.contains(slot))
            .collect()
    }

    fn look(used: &[u8], own: &[u8], claimed: bool, slot: u8) -> SlotMapMark {
        let used = used.contains(&slot);
        match (claimed, used) {
            (true, true) if !own.contains(&slot) => SlotMapMark::Overwritten,
            (true, _) => SlotMapMark::Claimed,
            (false, true) => SlotMapMark::Used,
            (false, false) => SlotMapMark::Free,
        }
    }

    fn overwritten(used: &[u8], own: &[u8], claimed: &[u8]) -> Vec<u8> {
        claimed
            .iter()
            .copied()
            .filter(|slot| used.contains(slot) && !own.contains(slot))
            .collect()
    }
}

fn suggestion(slot_use: &SlotUse, needs: &ExportNeeds) -> Claim {
    let free_sfx = SlotUse::free_or_own(&slot_use.used.sfx, &slot_use.own.sfx);
    let instrument_slots = &needs.instrument_slots;
    let phrase_sfx = needs.sfx.saturating_sub(instrument_slots.len());
    let mut sfx: Vec<u8> = free_sfx
        .iter()
        .copied()
        .filter(|slot| !instrument_slots.contains(slot))
        .take(phrase_sfx)
        .chain(
            instrument_slots
                .iter()
                .copied()
                .filter(|slot| free_sfx.contains(slot)),
        )
        .collect();
    sfx.sort_unstable();
    let free_patterns = SlotUse::free_or_own(&slot_use.used.patterns, &slot_use.own.patterns);
    let patterns = (needs.patterns > 0)
        .then(|| {
            free_patterns
                .windows(needs.patterns)
                .find(|run| run.windows(2).all(|pair| pair[0] + 1 == pair[1]))
                .map(|run| (run[0], run[run.len() - 1]))
        })
        .flatten();
    Claim {
        sfx,
        first_and_last_pattern: patterns,
    }
}

fn claimed_slots(claim: &Claim) -> Slots {
    let mut sfx = claim.sfx.clone();
    sfx.sort_unstable();
    sfx.dedup();
    Slots {
        sfx,
        patterns: claim
            .first_and_last_pattern
            .map_or_else(Vec::new, |(first, last)| (first..=last).collect()),
    }
}

fn room_check(needed: usize, claimed: usize, what: &str) -> (Severity, String) {
    let severity = if claimed >= needed {
        Severity::Info
    } else {
        Severity::Failure
    };
    (severity, format!("NEED {needed} {what}, HAVE {claimed}"))
}

fn overwrite_check(slots: &[u8], one: &str, many: &str) -> Option<(Severity, String)> {
    named_slots(slots, one, many).map(|named| (Severity::Warning, format!("OVERWRITES {named}")))
}

#[derive(Debug)]
pub struct CartView {
    field: CartField,
    sfx: ClaimText,
    patterns: ClaimText,
    opened_with: Option<TargetCart>,
    contents: TargetCartContents,
    focus_before: Focus,
}

impl CartView {
    pub fn new(target: Option<&TargetCart>, focus_before: Focus) -> Self {
        let claim = target
            .map(|target| target.claim.clone())
            .unwrap_or_default();
        Self {
            field: if target.is_some() {
                CartField::Sfx
            } else {
                CartField::Cart
            },
            sfx: ClaimText::sfx(&claimed_slots(&claim).sfx),
            patterns: ClaimText::patterns(claim.first_and_last_pattern),
            opened_with: target.cloned(),
            contents: TargetCartContents::Unknown,
            focus_before,
        }
    }

    pub fn field(&self) -> CartField {
        self.field
    }

    pub fn opened_with(&self) -> Option<&TargetCart> {
        self.opened_with.as_ref()
    }

    pub fn focus_before(&self) -> Focus {
        self.focus_before
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.column_and_row_delta();
        let index = FIELDS
            .iter()
            .position(|field| *field == self.field)
            .unwrap_or(0);
        self.field = FIELDS[stepped(index, rows, FIELDS.len())];
    }

    fn claim_text_under_cursor(&mut self) -> Option<&mut ClaimText> {
        match self.field {
            CartField::Cart => None,
            CartField::Sfx => Some(&mut self.sfx),
            CartField::Patterns => Some(&mut self.patterns),
        }
    }

    pub fn type_digit(&mut self, digit: u8) {
        if let Some(text) = self.claim_text_under_cursor() {
            text.type_digit(digit);
        }
    }

    pub fn type_dash(&mut self) {
        if let Some(text) = self.claim_text_under_cursor() {
            text.type_dash();
        }
    }

    pub fn backspace(&mut self) {
        if let Some(text) = self.claim_text_under_cursor() {
            text.backspace();
        }
    }

    pub fn typed_claim(&self, claim: &Claim) -> Claim {
        Claim {
            sfx: self.sfx.sfx_slots().unwrap_or_else(|| claim.sfx.clone()),
            first_and_last_pattern: self
                .patterns
                .pattern_range()
                .unwrap_or(claim.first_and_last_pattern),
        }
    }

    pub fn show_claim(&mut self, claim: &Claim) {
        self.sfx = ClaimText::sfx(&claimed_slots(claim).sfx);
        self.patterns = ClaimText::patterns(claim.first_and_last_pattern);
    }

    pub fn show_contents(&mut self, contents: Result<Cart, String>) {
        self.contents = match contents {
            Ok(cart) => TargetCartContents::Read(Box::new(cart)),
            Err(reason) => TargetCartContents::Unreadable(reason),
        };
    }

    pub fn suggested_claim(&self, hash: Option<&str>, needs: &ExportNeeds) -> Option<Claim> {
        match &self.contents {
            TargetCartContents::Read(cart) => Some(suggestion(&SlotUse::of(cart, hash), needs)),
            TargetCartContents::Unknown | TargetCartContents::Unreadable(_) => None,
        }
    }

    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        target: Option<&TargetCart>,
        needs: &ExportNeeds,
    ) {
        draw_title(grid, area.column, 0, "CART", focused);
        let claimed = claimed_slots(
            &target
                .map(|target| target.claim.clone())
                .unwrap_or_default(),
        );
        let path = target.map_or(NO_TARGET_CART, |target| target.path.as_str());
        let fields = [
            (CartField::Cart, CART_ROW, path_tail(path).to_owned()),
            (
                CartField::Sfx,
                SFX_HEADING_ROW + 1,
                self.sfx.text().to_owned(),
            ),
            (
                CartField::Patterns,
                PATTERNS_HEADING_ROW + 1,
                self.patterns.text().to_owned(),
            ),
        ];
        for (field, row, mut text) in fields {
            let picked = focused && field == self.field;
            if picked {
                grid.put(area.column, row, ">", POINTER);
                if field != CartField::Cart {
                    text.push('_');
                }
            }
            grid.put(area.column + 1, row, &text, VALUE);
        }
        let headings = [
            (SFX_HEADING_ROW, "SFX", claimed.sfx.len(), needs.sfx),
            (
                PATTERNS_HEADING_ROW,
                "PATTERNS",
                claimed.patterns.len(),
                needs.patterns,
            ),
        ];
        for (row, name, count, needed) in headings {
            grid.put(
                area.column,
                row,
                &format!("{name} {count}/{needed}"),
                HEADING,
            );
        }

        let slot_use = match &self.contents {
            TargetCartContents::Read(cart) => Some(SlotUse::of(
                cart,
                target.and_then(|target| target.claimed_bytes_hash.as_deref()),
            )),
            TargetCartContents::Unknown | TargetCartContents::Unreadable(_) => None,
        };
        let empty = Slots::default();
        let (used, own) = slot_use
            .as_ref()
            .map_or((&empty, &empty), |slot_use| (&slot_use.used, &slot_use.own));
        let maps = [
            (SFX_MAP_ROW, "SFX", &claimed.sfx, &used.sfx, &own.sfx),
            (
                PATTERN_MAP_ROW,
                "PATTERNS",
                &claimed.patterns,
                &used.patterns,
                &own.patterns,
            ),
        ];
        let mut overwritten = Vec::new();
        for (row, name, claimed, used, own) in maps {
            draw_map(grid, area, row, name, |slot| {
                SlotUse::look(used, own, claimed.contains(&slot), slot)
            });
            overwritten.push(SlotUse::overwritten(used, own, claimed));
        }

        let checks = self.checks(target, &claimed, needs, &overwritten);
        draw_checks(grid, area, &checks);
    }

    fn checks(
        &self,
        target: Option<&TargetCart>,
        claimed: &Slots,
        needs: &ExportNeeds,
        overwritten: &[Vec<u8>],
    ) -> Vec<(Severity, String)> {
        let mut checks = Vec::new();
        if target.is_none() {
            checks.push((
                Severity::Failure,
                format!("{NO_TARGET_CART}: ENTER PICKS ONE"),
            ));
            return checks;
        }
        if let TargetCartContents::Unreadable(reason) = &self.contents {
            checks.push((Severity::Failure, format!("CANNOT READ THE CART: {reason}")));
        }
        if !self.sfx.is_whole_claim() {
            checks.push((Severity::Failure, "SFX TEXT IS INCOMPLETE".to_owned()));
        }
        if !self.patterns.is_whole_claim() {
            checks.push((Severity::Failure, "PATTERN TEXT IS INCOMPLETE".to_owned()));
        }
        checks.push(room_check(needs.sfx, claimed.sfx.len(), "SFX"));
        for &slot in &needs.instrument_slots {
            if !claimed.sfx.contains(&slot) {
                let failure = ExportFailure::MissingInstrumentSlot {
                    instrument: custom_instrument_of_slot(slot),
                    slot,
                };
                checks.push((Severity::Failure, failure.to_string()));
            }
        }
        checks.push(room_check(
            needs.patterns,
            claimed.patterns.len(),
            "PATTERNS",
        ));
        let [sfx, patterns] = overwritten else {
            return checks;
        };
        checks.extend(overwrite_check(sfx, "SFX", "SFX"));
        checks.extend(overwrite_check(patterns, "PATTERN", "PATTERNS"));
        checks
    }
}

fn path_tail(path: &str) -> &str {
    let extra = path.chars().count().saturating_sub(PATH_WIDTH);
    path.char_indices()
        .nth(extra)
        .map_or("", |(at, _)| &path[at..])
}

fn draw_map(grid: &mut Grid, area: Area, row: usize, name: &str, look: impl Fn(u8) -> SlotMapMark) {
    grid.put(area.column, row, name, HEADING);
    for slot in 0..SLOTS as u8 {
        let (map_row, column) = (
            usize::from(slot) / MAP_COLUMNS,
            usize::from(slot) % MAP_COLUMNS,
        );
        let y = row + 1 + map_row;
        if column == 0 {
            grid.put(area.column, y, &format!("{map_row:X}"), HEADING);
        }
        let (mark, style) = look(slot).mark();
        grid.put(area.column + MAP_COLUMN + column, y, mark, style);
    }
}

fn draw_checks(grid: &mut Grid, area: Area, checks: &[(Severity, String)]) {
    let mut row = FIRST_CHECK_ROW;
    for (severity, text) in checks {
        let lines = wrapped(text, TEXT_WIDTH);
        if row + lines.len() > area.rows {
            break;
        }
        grid.put(
            area.column + TAG_COLUMN,
            row,
            tag(*severity),
            tag_style(*severity),
        );
        for line in lines {
            grid.put(area.column + TEXT_COLUMN, row, &line, VALUE);
            row += 1;
        }
    }
}
