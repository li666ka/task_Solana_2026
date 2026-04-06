use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};
use resource_manager::cpi::accounts::MintResource;
use resource_manager::cpi::mint_resource;
use resource_manager::program::ResourceManager;
use resource_manager::{GameConfig, Player};

declare_id!("pVedD9QGPD24kbxZ5kSGav96aNrb6gCeAqDCaK7WBBc");

/// Search cooldown in seconds
const SEARCH_COOLDOWN: i64 = 60;

/// Search Program
/// Allows players to search for random resources once every 60 seconds.
/// Each search yields 3 random resources minted via CPI to the Resource Manager.
#[program]
pub mod search {
    use super::*;

    /// Performs a resource search for the player.
    /// Generates 3 pseudo-random resources and mints them to the player's token accounts.
    /// Enforces a 60-second cooldown between searches.
    pub fn search_resources<'info>(ctx: Context<'_, '_, '_, 'info, SearchResources<'info>>) -> Result<()> {
        let player = &mut ctx.accounts.player;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // Check cooldown
        let time_diff = current_time - player.last_search_timestamp;
        require!(
            time_diff >= SEARCH_COOLDOWN,
            SearchError::SearchCooldownActive
        );

        // Update timestamp
        player.last_search_timestamp = current_time;

        // Generate 3 pseudo-random resource indices (0-5)
        let slot = clock.slot;
        let random_seed = current_time as u64 ^ slot;

        let resource1 = ((random_seed) % 6) as u8;
        let resource2 = ((random_seed / 7) % 6) as u8;
        let resource3 = ((random_seed / 13) % 6) as u8;

        let resources = [resource1, resource2, resource3];

        // Mint each resource via CPI to resource_manager
        let remaining = &ctx.remaining_accounts;
        // We expect pairs of (resource_mint, player_token_account) for each unique resource
        // For simplicity, we pass 6 mint accounts + 6 token accounts = 12 remaining accounts

        for &res_idx in resources.iter() {
            let mint_idx = (res_idx as usize) * 2;
            let token_idx = mint_idx + 1;

            if mint_idx + 1 < remaining.len() {
                let resource_mint = &remaining[mint_idx];
                let player_token_account = &remaining[token_idx];

                let cpi_accounts = MintResource {
                    game_config: ctx.accounts.game_config.to_account_info(),
                    resource_mint: resource_mint.clone(),
                    player_token_account: player_token_account.clone(),
                    authority: ctx.accounts.owner.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.resource_manager_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                mint_resource(cpi_ctx, res_idx, 1)?;
            }
        }

        msg!(
            "Search complete! Found resources: [{}, {}, {}]",
            resource1,
            resource2,
            resource3
        );
        Ok(())
    }
}

// ==================== ACCOUNTS ====================

#[derive(Accounts)]
pub struct SearchResources<'info> {
    #[account(
        mut,
        seeds = [b"player" as &[u8], owner.key().as_ref()],
        bump = player.bump,
        has_one = owner,
        seeds::program = resource_manager_program.key(),
    )]
    pub player: Account<'info, Player>,
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
        seeds::program = resource_manager_program.key(),
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub resource_manager_program: Program<'info, ResourceManager>,
    pub system_program: Program<'info, System>,
    // Remaining accounts: pairs of (resource_mint, player_token_account) × 6
}

// ==================== ERRORS ====================

#[error_code]
pub enum SearchError {
    #[msg("Search is on cooldown. Please wait 60 seconds between searches.")]
    SearchCooldownActive,
}
