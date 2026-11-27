use super::files::{Waiting, cannot_use, shown_name};
use super::focus::Focus;
use super::importing::TEXT_CART_EXTENSION;
use super::messages::Report;
use super::panes::cart_view::{CartField, CartView};
use super::panes::file_list::Purpose;
use super::paths::relative_path;
use super::{Action, App, Direction};
use crate::cart::Cart;
use crate::export;
use crate::project::{Claim, TargetCart};
use std::path::{Path, PathBuf};

const PICK_A_CART_FIRST: &str = "PICK A TARGET CART FIRST";

impl App {
    pub(super) fn cart_view_has_focus(&self) -> bool {
        self.overlays.cart_view_has_focus(self.focus)
    }

    pub(super) fn toggle_cart(&mut self) {
        if self.overlays.cart_view_open() {
            self.close_cart_view();
        } else {
            self.open_cart_view();
        }
    }

    pub(super) fn open_cart_view(&mut self) {
        self.cart_visits += 1;
        let view = CartView::new(self.project.target_cart(), self.played_pane);
        self.overlays.open_cart_view(view);
        self.set_focus(Focus::RightSlot);
        if let Some(target) = self.project.target_cart() {
            let path = self.target_cart_path(&target.path);
            self.request_bytes(path.clone());
            self.waiting = Some(Waiting::CartViewReading(path));
        }
    }

    pub(super) fn close_cart_view(&mut self) {
        let Some(view) = self.overlays.take_cart_view() else {
            return;
        };
        if self.focus == Focus::RightSlot && !self.overlays.log_open() {
            self.set_focus(view.focus_before());
        }
    }

    pub(super) fn use_cart_view(&mut self, action: Action) -> bool {
        let Some(view) = self.overlays.cart_view() else {
            return false;
        };
        match action {
            Action::Move(direction @ (Direction::Up | Direction::Down)) => {
                view.move_cursor(direction);
            }
            Action::HexDigit(digit) => self.edit_claim(|view| view.type_digit(digit)),
            Action::TypeChar('-') => self.edit_claim(CartView::type_dash),
            Action::Move(_) | Action::Clear | Action::TypeChar(_) => {}
            Action::Backspace => self.edit_claim(CartView::backspace),
            Action::SuggestClaim => self.suggest_claim(),
            Action::Confirm => match view.field() {
                CartField::Cart => self.browse(Purpose::TargetCart),
                CartField::Sfx => view.move_cursor(Direction::Down),
                CartField::Patterns => self.close_cart_view(),
            },
            Action::Cancel => {
                let opened_with = view.opened_with().cloned();
                self.project.set_target_cart(opened_with);
                self.close_cart_view();
            }
            Action::ToggleCart => self.close_cart_view(),
            _ => return false,
        }
        true
    }

    fn edit_claim(&mut self, edit: impl FnOnce(&mut CartView)) {
        let Some(view) = self.overlays.cart_view() else {
            return;
        };
        if view.field() == CartField::Cart {
            return;
        }
        let Some(mut target) = self.project.target_cart().cloned() else {
            self.remark = Some(PICK_A_CART_FIRST.into());
            return;
        };
        edit(view);
        target.claim = view.typed_claim(&target.claim);
        self.project.set_target_cart(Some(target));
    }

    fn suggest_claim(&mut self) {
        let Some(target) = self.project.target_cart() else {
            self.remark = Some(PICK_A_CART_FIRST.into());
            return;
        };
        self.apply_suggestion(target.clone());
    }

    fn apply_suggestion(&mut self, mut target: TargetCart) {
        let Some(view) = self.overlays.cart_view() else {
            return;
        };
        let needs = export::needs(&self.project);
        let hash = target.claimed_bytes_hash.as_deref();
        if let Some(claim) = view.suggested_claim(hash, &needs) {
            view.show_claim(&claim);
            target.claim = claim;
        }
        self.project.set_target_cart(Some(target));
    }

    pub(super) fn show_claim_in_cart_view(&mut self) {
        let claim = self
            .project
            .target_cart()
            .map(|target| target.claim.clone())
            .unwrap_or_default();
        if let Some(view) = self.overlays.cart_view() {
            view.show_claim(&claim);
        }
    }

    pub(super) fn show_cart_in_view(&mut self, contents: &[u8]) {
        if let Some(view) = self.overlays.cart_view() {
            view.show_contents(Cart::read(contents).map_err(|error| error.to_string()));
        }
    }

    pub(super) fn show_unreadable_cart_in_view(&mut self, reason: String) {
        if let Some(view) = self.overlays.cart_view() {
            view.show_contents(Err(reason));
        }
    }

    pub(super) fn path_from_project(&self, path: &Path) -> String {
        relative_path(self.target_cart_folder(), path)
            .unwrap_or_else(|| path.to_path_buf())
            .to_string_lossy()
            .into_owned()
    }

    pub(super) fn finish_picking_cart(&mut self, path: &Path, contents: &[u8]) {
        let cart = match Cart::read(contents) {
            Ok(cart) => cart,
            Err(error) => {
                self.overlays.take_file_list();
                self.report(Report::failure(cannot_use(path, &error.to_string())));
                return;
            }
        };
        let claimed_bytes_hash_of_same_cart = self
            .project
            .target_cart()
            .filter(|target| target.path == self.path_from_project(path))
            .and_then(|target| target.claimed_bytes_hash.clone());
        self.adopt_target_cart(path, cart, claimed_bytes_hash_of_same_cart);
    }

    fn adopt_target_cart(&mut self, path: &Path, cart: Cart, claimed_bytes_hash: Option<String>) {
        self.overlays.take_file_list();
        let target = TargetCart {
            path: self.path_from_project(path),
            claim: Claim::default(),
            claimed_bytes_hash,
        };
        if let Some(view) = self.overlays.cart_view() {
            view.show_contents(Ok(cart));
            view.move_cursor(Direction::Down);
        }
        self.apply_suggestion(target);
    }

    pub(super) fn new_cart_name(&self) -> String {
        self.project_path
            .as_deref()
            .and_then(Path::file_stem)
            .map_or_else(String::new, |stem| {
                format!("{}{TEXT_CART_EXTENSION}", stem.to_string_lossy())
            })
    }

    pub(super) fn make_new_cart(&mut self, path: PathBuf) {
        self.request_write_bytes(path.clone(), Cart::new_cart_bytes());
        self.waiting = Some(Waiting::MakingCart(path));
    }

    pub(super) fn finish_making_cart(&mut self, path: &Path) {
        let Ok(cart) = Cart::read(&Cart::new_cart_bytes()) else {
            return;
        };
        self.adopt_target_cart(path, cart.clone(), None);
        if let Some(target) = self.project.target_cart() {
            let hash = cart.claimed_bytes_hash(&target.claim);
            self.record_claimed_bytes_hash(hash);
        }
        self.report(Report::info(format!(
            "CREATED NEW CART {}",
            shown_name(path)
        )));
    }
}
