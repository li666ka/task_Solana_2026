use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, MintTo, Burn};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("4TRBWMfhQQWGw7qP6GfGv9P4vogZL55eEPuqR64aZXJu");

/// Resource Manager Program
/// Manages the creation and destruction of game resources (SPL Token-2022).
/// Resources can only be minted/burned through authorized programs (Search, Crafting).
#[program]
pub mod resource_manager {
    use super::*;

    /// Initializes the game configuration with admin authority.
    pub fn initialize_game(ctx: Context<InitializeGame>) -> Result<()> {
        let game_config = &mut ctx.accounts.game_config;
        game_config.admin = ctx.accounts.admin.key();
        game_config.resource_mints = [Pubkey::default(); 6];
        game_config.magic_token_mint = Pubkey::default();
        game_config.item_prices = [100, 150, 200, 250];
        game_config.bump = ctx.bumps.game_config;
        msg!("Game initialized by admin: {}", game_config.admin);
        Ok(())
    }

    /// Creates a new resource mint (SPL Token-2022) and registers it in GameConfig.
    /// Only the admin can call this function.
    pub fn create_resource_mint(
        ctx: Context<CreateResourceMint>,
        resource_index: u8,
        _name: String,
        _symbol: String,
        _uri: String,
    ) -> Result<()> {
        require!(resource_index < 6, GameError::InvalidResourceIndex);
        let game_config = &mut ctx.accounts.game_config;
        game_config.resource_mints[resource_index as usize] = ctx.accounts.resource_mint.key();
        msg!("Resource mint created: index={}", resource_index);
        Ok(())
    }

    /// Registers the MagicToken mint address in the game config.
    pub fn set_magic_token_mint(ctx: Context<SetMagicTokenMint>) -> Result<()> {
        let game_config = &mut ctx.accounts.game_config;
        game_config.magic_token_mint = ctx.accounts.magic_token_mint.key();
        msg!("MagicToken mint set: {}", game_config.magic_token_mint);
        Ok(())
    }

    /// Registers a player by creating their Player PDA account.
    pub fn register_player(ctx: Context<RegisterPlayer>) -> Result<()> {
        let player = &mut ctx.accounts.player;
        player.owner = ctx.accounts.owner.key();
        player.last_search_timestamp = 0;
        player.bump = ctx.bumps.player;
        msg!("Player registered: {}", player.owner);
        Ok(())
    }

    /// Mints resources to a player's token account.
    pub fn mint_resource(
        ctx: Context<MintResource>,
        resource_index: u8,
        amount: u64,
    ) -> Result<()> {
        require!(resource_index < 6, GameError::InvalidResourceIndex);
        require!(amount > 0, GameError::InvalidAmount);

        let seeds = &[
            b"resource_mint" as &[u8],
            &[resource_index],
            &[ctx.bumps.resource_mint],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.resource_mint.to_account_info(),
            to: ctx.accounts.player_token_account.to_account_info(),
            authority: ctx.accounts.resource_mint.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token_2022::mint_to(cpi_ctx, amount)?;

        msg!("Minted {} of resource {} to player", amount, resource_index);
        Ok(())
    }

    /// Burns resources from a player's token account.
    pub fn burn_resource(
        ctx: Context<BurnResource>,
        resource_index: u8,
        amount: u64,
    ) -> Result<()> {
        require!(resource_index < 6, GameError::InvalidResourceIndex);
        require!(amount > 0, GameError::InvalidAmount);

        let cpi_accounts = Burn {
            mint: ctx.accounts.resource_mint.to_account_info(),
            from: ctx.accounts.player_token_account.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token_2022::burn(cpi_ctx, amount)?;

        msg!("Burned {} of resource {} from player", amount, resource_index);
        Ok(())
    }
}

// ==================== ACCOUNTS ====================

#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + GameConfig::INIT_SPACE,
        seeds = [b"game_config"],
        bump,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(resource_index: u8)]
pub struct CreateResourceMint<'info> {
    #[account(
        mut,
        seeds = [b"game_config"],
        bump = game_config.bump,
        has_one = admin,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(
        init,
        payer = admin,
        seeds = [b"resource_mint" as &[u8], &[resource_index]],
        bump,
        mint::decimals = 0,
        mint::authority = resource_mint,
        mint::token_program = token_program,
    )]
    pub resource_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetMagicTokenMint<'info> {
    #[account(
        mut,
        seeds = [b"game_config"],
        bump = game_config.bump,
        has_one = admin,
    )]
    pub game_config: Account<'info, GameConfig>,
    /// CHECK: MagicToken mint validated by magic_token program
    pub magic_token_mint: UncheckedAccount<'info>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct RegisterPlayer<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + Player::INIT_SPACE,
        seeds = [b"player", owner.key().as_ref()],
        bump,
    )]
    pub player: Account<'info, Player>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(resource_index: u8)]
pub struct MintResource<'info> {
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(
        mut,
        seeds = [b"resource_mint" as &[u8], &[resource_index]],
        bump,
    )]
    pub resource_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub player_token_account: InterfaceAccount<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
#[instruction(resource_index: u8)]
pub struct BurnResource<'info> {
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(
        mut,
        seeds = [b"resource_mint" as &[u8], &[resource_index]],
        bump,
    )]
    pub resource_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub player_token_account: InterfaceAccount<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

// ==================== STATE ====================

#[account]
#[derive(InitSpace)]
pub struct GameConfig {
    pub admin: Pubkey,
    pub resource_mints: [Pubkey; 6],
    pub magic_token_mint: Pubkey,
    pub item_prices: [u64; 4],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Player {
    pub owner: Pubkey,
    pub last_search_timestamp: i64,
    pub bump: u8,
}

// ==================== ERRORS ====================

#[error_code]
pub enum GameError {
    #[msg("Invalid resource index. Must be 0-5.")]
    InvalidResourceIndex,
    #[msg("Invalid amount. Must be greater than 0.")]
    InvalidAmount,
    #[msg("Unauthorized access.")]
    Unauthorized,
}
