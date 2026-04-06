use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::{self, Token, MintTo};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("DenFbotSLY4toykAzrYjBxoViJSBipFyiEFQ4iSM2T2A");

/// Item NFT Program
/// Manages the creation of unique game item NFTs using Metaplex standard.
/// Items are created through the Crafting program and can be sold on the Marketplace.
#[program]
pub mod item_nft {
    use super::*;

    /// Creates a new NFT item for a player.
    /// Called via CPI from the Crafting program after resources are burned.
    pub fn create_item_nft(
        ctx: Context<CreateItemNft>,
        item_type: u8,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(item_type < 4, ItemNftError::InvalidItemType);

        // Save item metadata on-chain
        let item_metadata = &mut ctx.accounts.item_metadata;
        item_metadata.item_type = item_type;
        item_metadata.owner = ctx.accounts.player.key();
        item_metadata.mint = ctx.accounts.item_mint.key();
        item_metadata.bump = ctx.bumps.item_metadata;

        // Mint 1 token (NFT) to the player's token account
        let authority_bump = ctx.bumps.item_authority;
        let seeds = &[b"item_authority" as &[u8], &[authority_bump]];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.item_mint.to_account_info(),
            to: ctx.accounts.player_token_account.to_account_info(),
            authority: ctx.accounts.item_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token::mint_to(cpi_ctx, 1)?;

        // Create Metaplex metadata account via raw instruction
        let create_metadata_accounts = mpl_token_metadata::instructions::CreateMetadataAccountV3 {
            metadata: ctx.accounts.metadata_account.key(),
            mint: ctx.accounts.item_mint.key(),
            mint_authority: ctx.accounts.item_authority.key(),
            payer: ctx.accounts.player.key(),
            update_authority: (ctx.accounts.item_authority.key(), true),
            system_program: ctx.accounts.system_program.key(),
            rent: Some(ctx.accounts.rent.key()),
        };

        let create_metadata_args = mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
            data: mpl_token_metadata::types::DataV2 {
                name: name.clone(),
                symbol: symbol.clone(),
                uri: uri.clone(),
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            is_mutable: true,
            collection_details: None,
        };

        let metadata_ix = create_metadata_accounts.instruction(create_metadata_args);

        invoke_signed(
            &metadata_ix,
            &[
                ctx.accounts.metadata_account.to_account_info(),
                ctx.accounts.item_mint.to_account_info(),
                ctx.accounts.item_authority.to_account_info(),
                ctx.accounts.player.to_account_info(),
                ctx.accounts.item_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            signer_seeds,
        )?;

        // Create Master Edition (makes it a true NFT with supply of 1)
        let create_master_edition_accounts = mpl_token_metadata::instructions::CreateMasterEditionV3 {
            edition: ctx.accounts.master_edition.key(),
            mint: ctx.accounts.item_mint.key(),
            update_authority: ctx.accounts.item_authority.key(),
            mint_authority: ctx.accounts.item_authority.key(),
            metadata: ctx.accounts.metadata_account.key(),
            payer: ctx.accounts.player.key(),
            token_program: ctx.accounts.token_program.key(),
            system_program: ctx.accounts.system_program.key(),
            rent: Some(ctx.accounts.rent.key()),
        };

        let master_edition_args = mpl_token_metadata::instructions::CreateMasterEditionV3InstructionArgs {
            max_supply: Some(0),
        };

        let master_edition_ix = create_master_edition_accounts.instruction(master_edition_args);

        invoke_signed(
            &master_edition_ix,
            &[
                ctx.accounts.master_edition.to_account_info(),
                ctx.accounts.item_mint.to_account_info(),
                ctx.accounts.item_authority.to_account_info(),
                ctx.accounts.player.to_account_info(),
                ctx.accounts.metadata_account.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            signer_seeds,
        )?;

        msg!("Item NFT created: type={}, name={}", item_type, name);
        Ok(())
    }
}

// ==================== ACCOUNTS ====================

#[derive(Accounts)]
#[instruction(item_type: u8)]
pub struct CreateItemNft<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + ItemMetadata::INIT_SPACE,
        seeds = [b"item_metadata", item_mint.key().as_ref()],
        bump,
    )]
    pub item_metadata: Box<Account<'info, ItemMetadata>>,
    #[account(
        init,
        payer = player,
        mint::decimals = 0,
        mint::authority = item_authority,
        mint::freeze_authority = item_authority,
    )]
    pub item_mint: Box<Account<'info, anchor_spl::token::Mint>>,
    /// CHECK: PDA authority for NFT operations
    #[account(
        seeds = [b"item_authority"],
        bump,
    )]
    pub item_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = player,
        associated_token::mint = item_mint,
        associated_token::authority = player,
    )]
    pub player_token_account: Box<Account<'info, anchor_spl::token::TokenAccount>>,
    /// CHECK: Metaplex metadata account (validated by Metaplex program)
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    /// CHECK: Metaplex master edition account
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Metaplex Token Metadata Program
    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// ==================== STATE ====================

/// On-chain metadata for game items.
#[account]
#[derive(InitSpace)]
pub struct ItemMetadata {
    /// Item type (0=Saber, 1=Staff, 2=Armor, 3=Bracelet)
    pub item_type: u8,
    /// Current owner of the item
    pub owner: Pubkey,
    /// NFT mint address
    pub mint: Pubkey,
    /// PDA bump seed
    pub bump: u8,
}

// ==================== ERRORS ====================

#[error_code]
pub enum ItemNftError {
    #[msg("Invalid item type. Must be 0-3.")]
    InvalidItemType,
}
