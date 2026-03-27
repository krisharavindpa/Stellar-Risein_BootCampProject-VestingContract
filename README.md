Soroban Vesting dApp
A high-performance, decentralized token vesting platform built on Stellar Soroban. This project allows organizations to lock up SEP-41 tokens and distribute them to stakeholders (founders, investors, or employees) over a predefined period, ensuring transparency and trust through smart contracts.

🚀 Project Overview
Blockchain: Stellar (Testnet)

Smart Contract: Soroban (Rust SDK)

Frontend: Next.js 16 (App Router), React 19, Tailwind CSS

Wallet Integration: Freighter API

Developer: Krish Aravind PA

🛠 Features
Secure Initialization: One-time contract setup for admin and token addresses.

Automated Vesting: Linear or cliff-based token release schedules.

Admin Dashboard: Modern React 19 interface for managing contract state.

Type-Safe Bindings: Fully generated TypeScript bindings for seamless contract interaction.

📦 Installation & Setup
1. Prerequisites
Ensure you have the following installed:

Stellar CLI

Node.js 20+

Freighter Wallet extension in your browser.

2. Clone the Repository
Bash
git clone https://github.com/krisharavindpa/soroban-vesting-dapp.git
cd soroban-vesting-dapp
3. Install Dependencies
Bash
npm install
4. Environment Setup
Create a .env.local file in the root directory:

Code snippet
NEXT_PUBLIC_VESTING_CONTRACT_ID=your_contract_id_here
NEXT_PUBLIC_TOKEN_ADDRESS=CDLZFC3SYJYDZT7K67VZ75YJ3LPP2VXD535YY2G6S2Z7SFCY56267SU
5. Generate Contract Bindings
If you have updated the Rust contract, regenerate the TypeScript client:

Bash
stellar contract bindings typescript --network testnet --contract-id <YOUR_CONTRACT_ID> --output-dir src/contracts/vesting
6. Run the Development Server
Bash
npm run dev
Open http://localhost:3000 to view the Admin Panel.

📜 Development Plan
Contract Logic: Implement initialize, add_beneficiary, and claim functions in Rust.

State Management: Define storage for admin keys, token balances, and release timestamps.

Security: Implement access control to ensure only the admin can set vesting schedules.

Frontend Integration: Build the React 19 Admin component using useActionState for transaction handling.

Testing & Deployment: Verify logic on Stellar Testnet and deploy the finalized contract.

👨‍💻 Author
Krish Aravind PA Computer Science Engineering Student @ SRMIST KTR GitHub: @krisharavindpa
