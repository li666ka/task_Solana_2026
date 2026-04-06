use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, MintTo};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("AwByXhb1LdHFF278jvsAyFrc2vQa2yWgd5DixKV6VuAw");

/// MagicToken Program
/// Manages the MagicToken (SPL Token-2022) used as in-game currency.
/// MagicToken can ONLY be minted through the Marketplace program via CPI.
#[program]
pub mod magic_token {
    use super::*;

    /// Initializes the MagicToken mint.
    /// The mint authority is a PDA controlled by this program.
    pub fn initialize_mint(ctx: Context<InitializeMagicMint>) -> Result<()> {
        let config = &mut ctx.accounts.magic_config;
        config.mint = ctx.accounts.magic_mint.key();
        config.authority = ctx.accounts.mint_authority.key();
        config.bump = ctx.bumps.mint_authority;
        config.config_bump = ctx.bumps.magic_config;
        msg!("MagicToken mint initialized: {}", config.mint);
        Ok(())
    }

    /// Mints MagicToken to a player's token account.
    /// This function is intended to be called via CPI from the Marketplace program.
    pub fn mint_magic_token(ctx: Context<MintMagicToken>, amount: u64) -> Result<()> {
        require!(amount > 0, MagicTokenError::InvalidAmount);

        let seeds = &[
            b"magic_authority" as &[u8],
            &[ctx.accounts.magic_config.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.magic_mint.to_account_info(),
            to: ctx.accounts.player_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        token_2022::mint_to(cpi_ctx, amount)?;
        msg!("Minted {} MagicToken to player", amount);
        Ok(())
    }
}

// ==================== ACCOUNTS ====================

#[derive(Accounts)]
pub struct InitializeMagicMint<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + MagicConfig::INIT_SPACE,
        seeds = [b"magic_config"],
        bump,
    )]
    pub magic_config: Account<'info, MagicConfig>,
    #[account(
        init,
        payer = admin,
        seeds = [b"magic_mint"],
        bump,
        mint::decimals = 0,
        mint::authority = mint_authority,
        mint::token_program = token_program,
    )]
    pub magic_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: PDA used as mint authority
    #[account(
        seeds = [b"magic_authority"],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintMagicToken<'info> {
    #[account(
        seeds = [b"magic_config"],
        bump = magic_config.config_bump,
    )]
    pub magic_config: Account<'info, MagicConfig>,
    #[account(
        mut,
        address = magic_config.mint,
    )]
    pub magic_mint: InterfaceAccount<'info, Mint>,
    /// CHECK: PDA mint authority
    #[account(
        seeds = [b"magic_authority"],
        bump = magic_config.bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub player_token_account: InterfaceAccount<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

// ==================== STATE ====================

/// Configuration for the MagicToken program.
#[account]
#[derive(InitSpace)]
pub struct MagicConfig {
    /// The MagicToken mint address
    pub mint: Pubkey,
    /// The PDA authority that controls minting
    pub authority: Pubkey,
    /// Bump for the mint authority PDA
    pub bump: u8,
    /// Bump for this config PDA
    pub config_bump: u8,
}

// ==================== ERRORS ====================

#[error_code]
pub enum MagicTokenError {
    #[msg("Invalid amount. Must be greater than 0.")]
    InvalidAmount,
}
