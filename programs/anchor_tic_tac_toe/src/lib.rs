use anchor_lang::prelude::*;
use num_derive::*;
use num_traits::*;


declare_id!("2gDdGXuMg2QRm3ftmP1U8iDFXJTWNqah7DHaVWQ8jpRF");

#[program]
pub mod anchor_tic_tac_toe {
    use super::*;

    pub fn set_up_game(ctx: Context<SetUpGame>, player_two: Pubkey) -> Result<()> {
        ctx.accounts.game.start([ctx.accounts.player_one.key(), player_two])
      
    }

    pub fn play(ctx: Context<Play>, tile: Tile) -> Result<()> {
        let game = &mut ctx.accounts.game;

        //checks that the player is the player that is expected
        require_keys_eq!(
            game.current_player(),
            ctx.accounts.player.key(),
            TicTacToeError::NotPlayersTurn
        );
        game.play(&tile)
    }
}


// game logic
impl Game {
    pub const MAXIMUM_SIZE: usize = (32 * 2) + 1 + (9 * (1 + 1)) + (32 + 1);
// Pubkey has a length of 32 bytes so 2*32 = 64
// u8 as a vector has a length of 1
// the board has a length of (9 * (1 + 1)). We know the board has 9 tiles (-> 9) of type Option which borsh serializes with 1 byte (set to 1 for Some and 0 for None) 
//plus the size of whatever's in the Option. In this case, it's a simple enum with types that don't hold more types so the maximum size of the enum is also just 1 (for its discriminant). In total that means we get 9 (tiles) * (1 (Option) + 1(Sign discriminant)).
// state is also an enum so we need 1 byte for the discriminant. We have to init the account with the maximum size and the maximum size of an enum is the size of its biggest variant. In this case that's the winner variant which holds a Pubkey. A Pubkey is 32 bytes long so the size of state is 1 (discriminant) + 32 (winner pubkey) (MAXIMUM_SIZE is a const variable so specifying it in terms of a sum of the sizes of Game's members' fields does not incur any runtime cost).
// In addition to the game's size, we have to add another 8 to the space. This is space for the internal discriminator which anchor sets automatically. In short, the discriminator is how anchor can differentiate between different accounts of the same program. For more information, check out the Anchor space reference.

    pub fn start(&mut self, players: [Pubkey; 2]) -> Result<()> {
        require_eq!(self.turn, 0, TicTacToeError::GameAlreadyStarted);
        self.players = players;
        self.turn = 1;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.state == GameState::Active
    }

    fn current_player_index(&self) -> usize {
        ((self.turn - 1) % 2) as usize
    }

    pub fn current_player(&self) -> Pubkey {
        self.players[self.current_player_index()]
    }

    pub fn play(&mut self, tile: &Tile) -> Result<()> {
        require!(self.is_active(), TicTacToeError::GameAlreadyOver);

        match tile {
            tile @ Tile {
                row: 0..=2,
                column: 0..=2,
            } => match self.board[tile.row as usize][tile.column as usize] {
                Some(_) => return Err(TicTacToeError::TileAlreadySet.into()),
                None => {
                    self.board[tile.row as usize][tile.column as usize] =
                        Some(Sign::from_usize(self.current_player_index()).unwrap());
                }
            },
            _ => return Err(TicTacToeError::TileOutOfBounds.into()),
        }

        self.update_state();

        if GameState::Active == self.state {
            self.turn += 1;
        }

        Ok(())
    }

    fn is_winning_trio(&self, trio: [(usize, usize); 3]) -> bool {
        let [first, second, third] = trio;
        self.board[first.0][first.1].is_some()
            && self.board[first.0][first.1] == self.board[second.0][second.1]
            && self.board[first.0][first.1] == self.board[third.0][third.1]
    }

    fn update_state(&mut self) {
        for i in 0..=2 {
            // three of the same in one row
            if self.is_winning_trio([(i, 0), (i, 1), (i, 2)]) {
                self.state = GameState::Won {
                    winner: self.current_player(),
                };
                return;
            }
            // three of the same in one column
            if self.is_winning_trio([(0, i), (1, i), (2, i)]) {
                self.state = GameState::Won {
                    winner: self.current_player(),
                };
                return;
            }
        }

        // three of the same in one diagonal
        if self.is_winning_trio([(0, 0), (1, 1), (2, 2)])
            || self.is_winning_trio([(0, 2), (1, 1), (2, 0)])
        {
            self.state = GameState::Won {
                winner: self.current_player(),
            };
            return;
        }

        // reaching this code means the game has not been won,
        // so if there are unfilled tiles left, it's still active
        for row in 0..=2 {
            for column in 0..=2 {
                if self.board[row][column].is_none() {
                    return;
                }
            }
        }

        // game has not been won
        // game has no more free tiles
        // -> game ends in a tie
        self.state = GameState::Tie;
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct Tile {
    row: u8,
    column: u8,
}

#[derive(Accounts)]
pub struct SetUpGame<'info> {
    #[account(init, payer = player_one, space = 8 + Game::MAXIMUM_SIZE)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub player_one: Signer<'info>,
    pub system_program: Program<'info, System>,
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum GameState {
    Active,
    Tie,
    Won {winner: Pubkey},
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    pub player: Signer<'info>,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    FromPrimitive,
    ToPrimitive,
    Copy,
    Clone,
    PartialEq,
    Eq
)]
pub enum Sign {
    X,
    O,
}

#[error_code]
pub enum TicTacToeError {
    TileOutOfBounds,
    TileAlreadySet,
    GameAlreadyStarted,
    GameAlreadyOver,
    NotPlayersTurn,
}


#[account]
pub struct Game {
    players: [Pubkey; 2],
    turn: u8,
    board: [[Option<Sign>; 3]; 3],
    state: GameState,
}