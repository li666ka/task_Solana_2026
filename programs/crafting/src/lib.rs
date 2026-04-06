use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token::Token;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint as Mint2022, TokenAccount as TokenAccount2022};
use resource_manager::cpi::accounts::BurnResource;
use resource_manager::cpi::burn_resource;
use resource_manager::program::ResourceManager;
use resource_manager::GameConfig;
use item_nft::cpi::accounts::CreateItemNft;
use item_nft::cpi::create_item_nft;
use item_nft::program::ItemNft;

declare_id!("6VgPYQdeVUXqU7yhZXsamUCmCngXCm5jRB3UU16pm5U8");

/// Crafting recipes: [WOOD, IRON, GOLD, LEATHER, STONE, DIAMOND]
const RECIPES: [[u64; 6]; 4] = [
    [1, 3, 0, 1, 0, 0], // Шабля козака: 3 Iron + 1 Wood + 1 Leather
    [2, 0, 1, 0, 0, 1], // Посох старійшини: 2 Wood + 1 Gold + 1 Diamond
    [0, 2, 1, 4, 0, 0], // Броня характерника: 4 Leather + 2 Iron + 1 Gold
    [0, 4, 2, 0, 0, 2], // Бойовий браслет: 4 Iron + 2 Gold + 2 Diamond
];

/// Item names for NFT metadata
const ITEM_NAMES: [&str; 4] = [
    "Cossack Saber",
    "Elder Staff",
    "Character Armor",
    "Battle Bracelet",
];

const ITEM_SYMBOLS: [&str; 4] = ["SABER", "STAFF", "ARMOR", "BRACE"];

/// Crafting Program
/// Allows players to combine resources into unique NFT items.
/// Burns required resources via CPI and creates NFT via CPI.
#[program]
pub mod crafting {
    use super::*;

    /// Crafts an item by burning the required resources and minting an NFT.
    /// `item_type`: 0=Saber, 1=Staff, 2=Armor, 3=Bracelet
    pub fn craft_item<'info>(ctx: Context<'_, '_, '_, 'info, CraftItem<'info>>, item_type: u8) -> Result<()> {
        require!(item_type < 4, CraftingError::InvalidItemType);

        let recipe = RECIPES[item_type as usize];

        // Burn required resources via CPI
        // Remaining accounts contain pairs of (resource_mint, player_token_account) × 6
        let remaining = &ctx.remaining_accounts;
        require!(remaining.len() >= 12, CraftingError::InsufficientAccounts);

        for i in 0..6u8 {
            let required = recipe[i as usize];
            if required > 0 {
                let mint_idx = (i as usize) * 2;
                let token_idx = mint_idx + 1;

                let cpi_accounts = BurnResource {
                    game_config: ctx.accounts.game_config.to_account_info(),
                    resource_mint: remaining[mint_idx].clone(),
                    player_token_account: remaining[token_idx].clone(),
                    owner: ctx.accounts.player.to_account_info(),
                    token_program: ctx.accounts.token_2022_program.to_account_info(),
                };
                let cpi_program = ctx.accounts.resource_manager_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                burn_resource(cpi_ctx, i, required)?;
            }
        }

        // Create NFT via CPI to item_nft program
        let cpi_accounts = CreateItemNft {
            item_metadata: ctx.accounts.item_metadata.to_account_info(),
            item_mint: ctx.accounts.item_mint.to_account_info(),
            item_authority: ctx.accounts.item_authority.to_account_info(),
            player_token_account: ctx.accounts.player_nft_account.to_account_info(),
            metadata_account: ctx.accounts.metadata_account.to_account_info(),
            master_edition: ctx.accounts.master_edition.to_account_info(),
            player: ctx.accounts.player.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
            token_metadata_program: ctx.accounts.token_metadata_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.item_nft_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        let name = ITEM_NAMES[item_type as usize].to_string();
        let symbol = ITEM_SYMBOLS[item_type as usize].to_string();
        let uri = format!("https://cossack-business.io/items/{}.json", item_type);

        create_item_nft(cpi_ctx, item_type, name.clone(), symbol, uri)?;

        msg!("Crafted item: {} (type={})", name, item_type);
        Ok(())
    }
}

// ==================== ACCOUNTS ====================

#[derive(Accounts)]
#[instruction(item_type: u8)]
pub struct CraftItem<'info> {
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
        seeds::program = resource_manager_program.key(),
    )]
    pub game_config: Account<'info, GameConfig>,

    // NFT accounts
    /// CHECK: item_metadata created by item_nft program
    #[account(mut)]
    pub item_metadata: UncheckedAccount<'info>,
    #[account(mut)]
    pub item_mint: Signer<'info>,
    /// CHECK: PDA from item_nft program
    pub item_authority: UncheckedAccount<'info>,
    /// CHECK: Player's NFT token account
    #[account(mut)]
    pub player_nft_account: UncheckedAccount<'info>,
    /// CHECK: Metaplex metadata account
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,
    /// CHECK: Metaplex master edition
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,

    #[account(mut)]
    pub player: Signer<'info>,

    // Programs
    pub resource_manager_program: Program<'info, ResourceManager>,
    pub item_nft_program: Program<'info, ItemNft>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Metaplex Token Metadata program
    pub token_metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    // Remaining accounts: 12 accounts = 6 × (resource_mint + player_token_account)
}

// ==================== ERRORS ====================

#[error_code]
pub enum CraftingError {
    #[msg("Invalid item type. Must be 0-3.")]
    InvalidItemType,
    #[msg("Insufficient remaining accounts for crafting.")]
    InsufficientAccounts,
}
