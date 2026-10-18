//! Client-side state for the current round's board: the birds drawn for the round and a single
//! player's guesses about them.
//!
//! The board doesn't know whether it's in a solo game or a room. Either way, the next round's
//! birds are handed to [`BoardState::queue_round`], and the player moves on with
//! [`BoardState::advance_round`] once they've seen their results.

use crate::birds::{count_correct, Bird, BirdImage, Recording, RoundSetup};

pub struct BoardState {
    /// the round being played, or 0 before the first round has arrived
    pub round: usize,

    /// the round after this one, once its birds have arrived
    pub next_round: Option<(usize, RoundSetup)>,

    /// the recordings drawn for this round, indexed by bird
    pub recordings: Vec<Recording>,

    /// images of the birds drawn this round, indexed by bird
    pub images: Vec<BirdImage>,

    /// what sounds are in each sound box
    pub sound_positions: Vec<usize>,

    /// what sounds the user guessed for each sound box
    pub sound_guesses: Vec<Option<usize>>,

    /// what sound box the user has selected, for guessing
    pub selected_sound: Option<usize>,

    /// what sounds the user submitted for each sound box
    pub submitted: Option<Vec<usize>>,

    /// number of rounds where the user guessed [i] sounds correctly
    pub correct_guesses_per_round: Vec<u32>,
}

impl BoardState {
    pub fn new() -> Self {
        Self {
            round: 0,
            next_round: None,
            recordings: Vec::new(),
            images: Vec::new(),
            sound_positions: Vec::new(),
            sound_guesses: vec![None, None, None],
            selected_sound: None,
            submitted: None,
            correct_guesses_per_round: vec![0; 4],
        }
    }

    /// Whether a round has been loaded yet.
    pub fn has_round(&self) -> bool {
        self.round > 0
    }

    /// Drops the current round, e.g. when switching from solo play to a room. Keeps the score.
    pub fn clear_round(&mut self) {
        let correct_guesses_per_round = std::mem::take(&mut self.correct_guesses_per_round);
        *self = Self {
            correct_guesses_per_round,
            ..Self::new()
        };
    }

    /// Hands the board the birds for round `round`. The first round starts right away; later
    /// rounds wait in `next_round` until [`Self::advance_round`]. Rounds the board already has are
    /// ignored, so it's fine to call this again with the same round.
    pub fn queue_round(&mut self, round: usize, setup: RoundSetup) {
        let already_queued = self
            .next_round
            .as_ref()
            .is_some_and(|(queued, _)| *queued >= round);
        if round <= self.round || already_queued {
            return;
        }

        if self.has_round() {
            self.next_round = Some((round, setup));
        } else {
            self.start_round(round, setup);
        }
    }

    /// Moves on to the queued round, if it has arrived.
    pub fn advance_round(&mut self) {
        if let Some((round, setup)) = self.next_round.take() {
            self.start_round(round, setup);
        }
    }

    /// Loads a new round's birds and sound box order, and clears the previous round's guesses.
    fn start_round(&mut self, round: usize, setup: RoundSetup) {
        self.round = round;
        (self.recordings, self.images) = setup.birds.into_iter().map(Bird::into_parts).unzip();
        self.sound_positions = setup.sound_positions;

        self.sound_guesses = vec![None, None, None];
        self.selected_sound = None;
        self.submitted = None;
    }

    /// The recording playing in sound box `audio_box`.
    pub fn recording_for_box(&self, audio_box: usize) -> &Recording {
        &self.recordings[self.sound_positions[audio_box]]
    }

    /// Assigns the bird image `image_id` as the guess for the currently selected sound, if any.
    pub fn guess_selected(&mut self, image_id: usize) {
        if let Some(sound_id) = self.selected_sound {
            self.sound_guesses[sound_id] = Some(image_id);
            self.selected_sound = None;
        }
    }

    /// Locks in `guesses` and records how many of them were correct.
    pub fn submit(&mut self, guesses: Vec<usize>) {
        let correct_guesses = count_correct(&guesses, &self.sound_positions);

        self.submitted = Some(guesses);
        self.correct_guesses_per_round[correct_guesses] += 1;
        self.selected_sound = None;
    }
}
