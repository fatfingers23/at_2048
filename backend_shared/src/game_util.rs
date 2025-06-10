use thiserror::Error;
use twothousand_forty_eight::unified::hash::Hashable;
use twothousand_forty_eight::unified::validation::Validatable;
use twothousand_forty_eight::v2::io::SeededRecordingParseError;
use twothousand_forty_eight::v2::recording::SeededRecording;
use twothousand_forty_eight::v2::replay::MoveReplayError;

#[derive(Debug, Error)]
pub enum GameResultErrors {
    #[error("Failed to parse game data")]
    ParseError(SeededRecordingParseError),
    #[error("Move replay error: {0}")]
    MoveReplayError(MoveReplayError),
    #[error("Game score cannot be zero")]
    ZeroScore,
}

pub struct GameValidationResults {
    pub score: usize,
    pub hash: String,
}

pub fn parse_game_and_validate(game: &String) -> Result<GameValidationResults, GameResultErrors> {
    let history: SeededRecording = game.parse().map_err(|e| GameResultErrors::ParseError(e))?;

    match history.validate() {
        Ok(valid_history) => {
            if valid_history.score > 0 {
                Ok(GameValidationResults {
                    score: valid_history.score,
                    hash: history.game_hash(),
                })
            } else {
                Err(GameResultErrors::ZeroScore)
            }
        }
        Err(e) => Err(GameResultErrors::MoveReplayError(e)),
    }
}
