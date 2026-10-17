//! Client-side state for the current round's board: the birds drawn for the round and a single
//! player's guesses about them.

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::birds::{Bird, BirdImage, Recording};

pub struct BoardState {
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
            recordings: Vec::new(),
            images: Vec::new(),
            sound_positions: Vec::new(),
            sound_guesses: vec![None, None, None],
            selected_sound: None,
            submitted: None,
            correct_guesses_per_round: vec![0; 4],
        }
    }

    /// Loads the birds for a new round and reshuffles which recording goes in which sound box.
    pub fn start_round(&mut self, birds: Vec<Bird>) {
        (self.recordings, self.images) = birds.into_iter().map(Bird::into_parts).unzip();

        self.sound_positions = (0..self.recordings.len()).collect();
        self.sound_positions.shuffle(&mut thread_rng());
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
        let correct_guesses = guesses
            .iter()
            .zip(&self.sound_positions)
            .filter(|(guess, position)| guess == position)
            .count();

        self.submitted = Some(guesses);
        self.correct_guesses_per_round[correct_guesses] += 1;
        self.selected_sound = None;
    }

    pub fn reset_for_new_round(&mut self) {
        self.sound_guesses = vec![None, None, None];
        self.submitted = None;
    }
}
