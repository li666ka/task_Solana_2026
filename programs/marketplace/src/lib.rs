use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Burn as TokenBurn};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::TokenAccount as TokenAccount2022;
use magic_token::cpi::accounts::MintMagicToken;
use magic_token::cpi::mint_magic_token;
use magic_token::program::MagicToken;
use magic_token::MagicConfig;
use resource_manager::GameConfig;
use item_nft::ItemMetadata;

declare_id!("Hr7oW353Qve8fhGxB5J9iLTtKT9mNDbYfgNVieouCwa2");

#[program]
pub mod marketplace {
    use super::*;

    pub fn list_item(ctx: Context<ListItem>, price: u64) -> Result<()> {
        require!(price > 0, MarketplaceError::InvalidPrice);

        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.item_mint = ctx.accounts.item_mint.key();
        listing.item_type = ctx.accounts.item_metadata.item_type;
        listing.price = price;
        listing.is_active = true;
        listing.bump = ctx.bumps.listing;

        let cpi_accounts = token::Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token::transfer(cpi_ctx, 1)?;

        msg!("Item listed: price={} MagicToken", price);
        Ok(())
    }

    pub fn buy_item(ctx: Context<BuyItem>) -> Result<()> {
        let price = ctx.accounts.listing.price;
        let listing_bump = ctx.accounts.listing.bump;
        let item_mint_key = ctx.accounts.listing.item_mint;

        require!(ctx.accounts.listing.is_active, MarketplaceError::ListingNotActive);
        ctx.accounts.listing.is_active = false;

        let listing_seeds = &[
            b"listing" as &[u8],
            item_mint_key.as_ref(),
            &[listing_bump],
        ];
        let signer_seeds = &[&listing_seeds[..]];

        // Burn NFT from escrow
        let burn_accounts = TokenBurn {
            mint: ctx.accounts.item_mint.to_account_info(),
            from: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let burn_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            burn_accounts,
            signer_seeds,
        );
        token::burn(burn_ctx, 1)?;

        // Mint MagicToken to seller
        let cpi_accounts = MintMagicToken {
            magic_config: ctx.accounts.magic_config.to_account_info(),
            magic_mint: ctx.accounts.magic_mint.to_account_info(),
            mint_authority: ctx.accounts.magic_mint_authority.to_account_info(),
            player_token_account: ctx.accounts.seller_magic_token_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
            token_program: ctx.accounts.token_2022_program.to_account_info(),
        };
        let cpi_program = ctx.accounts.magic_token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        mint_magic_token(cpi_ctx, price)?;

        msg!("Item sold! Seller receives {} MagicToken", price);
        Ok(())
    }

    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        let listing_bump = ctx.accounts.listing.bump;
        let item_mint_key = ctx.accounts.listing.item_mint;

        require!(ctx.accounts.listing.is_active, MarketplaceError::ListingNotActive);
        ctx.accounts.listing.is_active = false;

        let listing_seeds = &[
            b"listing" as &[u8],
            item_mint_key.as_ref(),
            &[listing_bump],
        ];
        let signer_seeds = &[&listing_seeds[..]];

        let cpi_accounts = token::Transfer {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.seller_token_account.to_account_info(),
            authority: ctx.accounts.listing.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::transfer(cpi_ctx, 1)?;

        msg!("Listing cancelled for mint: {}", item_mint_key);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ListItem<'info> {
    #[account(
        init, payer = seller,
        space = 8 + Listing::INIT_SPACE,
        seeds = [b"listing" as &[u8], item_mint.key().as_ref()], bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        seeds = [b"item_metadata" as &[u8], item_mint.key().as_ref()],
        bump = item_metadata.bump,
        seeds::program = item_nft::ID,
    )]
    pub item_metadata: Account<'info, ItemMetadata>,
    pub item_mint: Account<'info, anchor_spl::token::Mint>,
    #[account(mut, associated_token::mint = item_mint, associated_token::authority = seller)]
    pub seller_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    #[account(mut)]
    pub seller: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyItem<'info> {
    #[account(
        mut, seeds = [b"listing" as &[u8], listing.item_mint.as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,
    #[account(mut)]
    pub item_mint: Account<'info, anchor_spl::token::Mint>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    #[account(mut)]
    pub seller_magic_token_account: InterfaceAccount<'info, TokenAccount2022>,
    #[account(
        seeds = [b"magic_config" as &[u8]],
        bump = magic_config.config_bump,
        seeds::program = magic_token_program.key(),
    )]
    pub magic_config: Account<'info, MagicConfig>,
    #[account(mut)]
    /// CHECK: MagicToken mint
    pub magic_mint: UncheckedAccount<'info>,
    /// CHECK: MagicToken mint authority PDA
    pub magic_mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub magic_token_program: Program<'info, MagicToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelListing<'info> {
    #[account(
        mut, seeds = [b"listing" as &[u8], listing.item_mint.as_ref()],
        bump = listing.bump, has_one = seller,
    )]
    pub listing: Account<'info, Listing>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    #[account(mut, associated_token::mint = item_mint, associated_token::authority = seller)]
    pub seller_token_account: Account<'info, anchor_spl::token::TokenAccount>,
    pub item_mint: Account<'info, anchor_spl::token::Mint>,
    #[account(mut)]
    pub seller: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub seller: Pubkey,
    pub item_mint: Pubkey,
    pub item_type: u8,
    pub price: u64,
    pub is_active: bool,
    pub bump: u8,
}

#[error_code]
pub enum MarketplaceError {
    #[msg("Invalid price. Must be greater than 0.")]
    InvalidPrice,
    #[msg("Listing is not active.")]
    ListingNotActive,
}
