# Гра "Козацький бізнес" — Solana

## Опис

Блокчейн-гра на Solana, де гравці шукають ресурси, крафтять унікальні предмети (NFT) та торгують ними на маркетплейсі за MagicToken.

## Архітектура

Гра складається з 6 програм (смарт-контрактів):

| Програма | Program ID | Призначення |
|---|---|---|
| `resource_manager` | `<DEPLOY_ADDRESS>` | Керування ресурсами (SPL Token-2022) |
| `magic_token` | `<DEPLOY_ADDRESS>` | MagicToken — внутрішня валюта |
| `search` | `<DEPLOY_ADDRESS>` | Пошук ресурсів (таймер 60с) |
| `item_nft` | `<DEPLOY_ADDRESS>` | Створення NFT предметів (Metaplex) |
| `crafting` | `<DEPLOY_ADDRESS>` | Крафт предметів з ресурсів |
| `marketplace` | `<DEPLOY_ADDRESS>` | Купівля/продаж за MagicToken |

## Ресурси (SPL Token-2022)

| ID | Назва | Символ | Decimals |
|---|---|---|---|
| 0 | Дерево | WOOD | 0 |
| 1 | Залізо | IRON | 0 |
| 2 | Золото | GOLD | 0 |
| 3 | Шкіра | LEATHER | 0 |
| 4 | Камінь | STONE | 0 |
| 5 | Алмаз | DIAMOND | 0 |

## Предмети (NFT через Metaplex)

| Предмет | Рецепт |
|---|---|
| Шабля козака | 3× Залізо + 1× Дерево + 1× Шкіра |
| Посох старійшини | 2× Дерево + 1× Золото + 1× Алмаз |
| Броня характерника | 4× Шкіра + 2× Залізо + 1× Золото |
| Бойовий браслет | 4× Залізо + 2× Золото + 2× Алмаз |

## Вимоги

- Rust 1.75+
- Solana CLI 1.18+
- Anchor CLI 0.30.1+
- Node.js 18+

## Встановлення та деплой

```bash
# 1. Клонувати репозиторій
git clone <repo-url>
cd cossack_business

# 2. Встановити залежності
npm install

# 3. Налаштувати Solana на Devnet
solana config set --url https://api.devnet.solana.com
solana-keygen new  # якщо немає гаманця
solana airdrop 5

# 4. Зібрати програми
anchor build

# 5. Оновити Program ID
# Після першого білду, замінити ID у declare_id!() кожної програми
# на реальні адреси з anchor keys list
anchor keys list

# 6. Перебілдити з правильними ID
anchor build

# 7. Задеплоїти на Devnet
anchor deploy

# 8. Запустити тести
anchor test
```

## Приклади взаємодії

### Ініціалізація гри (admin)
```typescript
await resourceManager.methods
  .initializeGame()
  .accounts({ gameConfig, admin, systemProgram })
  .rpc();
```

### Реєстрація гравця
```typescript
await resourceManager.methods
  .registerPlayer()
  .accounts({ player: playerPda, owner: playerWallet, systemProgram })
  .rpc();
```

### Пошук ресурсів
```typescript
await search.methods
  .searchResources()
  .accounts({ player, gameConfig, owner, tokenProgram, resourceManagerProgram, systemProgram })
  .remainingAccounts(resourceMintAndTokenAccountPairs)
  .rpc();
```

### Крафт предмета
```typescript
await crafting.methods
  .craftItem(0) // 0 = Шабля козака
  .accounts({ gameConfig, itemMetadata, itemMint, ... })
  .remainingAccounts(resourceMintAndTokenAccountPairs)
  .rpc();
```

### Продаж на маркетплейсі
```typescript
await marketplace.methods
  .listItem(new BN(100)) // 100 MagicToken
  .accounts({ listing, itemMetadata, itemMint, sellerTokenAccount, escrowTokenAccount, seller, ... })
  .rpc();
```

## Безпека

- Всі мінти контролюються через PDA (Program Derived Addresses)
- Прямий мінтинг/спалення токенів заборонено
- MagicToken мінтиться виключно через Marketplace (CPI)
- Таймер пошуку (60с) реалізований он-чейн через PDA з timestamp
- Перевірка підписантів у кожній транзакції

## Структура PDA

```
game_config:     seeds = ["game_config"]
player:          seeds = ["player", owner_pubkey]
resource_mint:   seeds = ["resource_mint", resource_index]
magic_config:    seeds = ["magic_config"]
magic_mint:      seeds = ["magic_mint"]
magic_authority: seeds = ["magic_authority"]
item_authority:  seeds = ["item_authority"]
item_metadata:   seeds = ["item_metadata", item_mint_pubkey]
listing:         seeds = ["listing", item_mint_pubkey]
```

## Тестування

```bash
anchor test
```

Тести покривають:
- ✅ Ініціалізація GameConfig
- ✅ Створення всіх 6 ресурсних мінтів
- ✅ Реєстрація гравця
- ✅ Мінтинг/спалення ресурсів
- ✅ Ініціалізація MagicToken
- ✅ Пошук ресурсів з таймером 60с
- ✅ Створення NFT предметів
- ✅ Листинг/купівля/скасування на Marketplace
- ✅ Перевірка прав доступу (PDA authority)
- ✅ Захист від неавторизованого доступу

## Автор

Студент НаУКМА — Завдання WhiteBIT 2026
