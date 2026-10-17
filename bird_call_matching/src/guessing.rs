//! Client-side state for a single player's guessing game.

use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Guessing {
    // what sounds are in each sound box
    pub sound_positions: Vec<usize>,

    // what sounds the user guessed for each sound box
    pub sound_guesses: Vec<Option<usize>>,

    // what sound box the user has selected, for guessing
    pub selected_sound: Option<usize>,

    // what sounds the user submitted for each sound box
    pub submitted: Option<Vec<usize>>,

    // number of rounds where the user guessed [i] sounds correctly
    pub correct_guesses_per_round: Vec<u32>,
}

impl Guessing {
    pub fn new() -> Self {
        let mut sound_positions = vec![0, 1, 2, 3, 4];
        sound_positions.shuffle(&mut thread_rng());
        Self {
            sound_positions,
            sound_guesses: vec![None, None, None],
            selected_sound: None,
            submitted: None,
            correct_guesses_per_round: vec![0; 4],
        }
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
        self.sound_guesses = vec![None, None, None, None, None];
        self.submitted = None;
    }
}
