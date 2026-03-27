"use client";

import React, {
  useState,
  useEffect,
  useActionState,
  useOptimistic,
  useTransition,
  useCallback,
} from "react";
import * as StellarSdk from "@stellar/stellar-sdk";
import { SorobanRpc } from "@stellar/stellar-sdk";
import {
  isConnected,
  isAllowed,
  requestAccess,
  getAddress,
  signTransaction,
} from "@stellar/freighter-api";

// ─── Generated contract bindings ────────────────────────────────────────────
import { Client as VestingClient } from "@/contracts/vesting/src/index";

// ─── Constants ───────────────────────────────────────────────────────────────
const CONTRACT_ID =
  process.env.NEXT_PUBLIC_VESTING_CONTRACT_ID ??
  "CB0SKGLRKLRDLDBHNWCJXKCCPSOLJY3KX27QUBOKZPIB0WHIIH22KM2A";
const TOKEN_ADDRESS =
  process.env.NEXT_PUBLIC_TOKEN_ADDRESS ??
  "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
const RPC_URL =
  process.env.SOROBAN_RPC_URL ?? "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE =
  process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE ??
  "Test SDF Network ; September 2015";
const TOKEN_DECIMALS = 7;

// ─── Helpers ─────────────────────────────────────────────────────────────────
const STROOPS = 10 ** TOKEN_DECIMALS;

function toDisplay(raw: bigint | number | string): string {
  const n = BigInt(raw);
  const whole = n / BigInt(STROOPS);
  const frac = n % BigInt(STROOPS);
  const fracStr = frac.toString().padStart(TOKEN_DECIMALS, "0").replace(/0+$/, "");
  return fracStr ? `${whole}.${fracStr}` : whole.toString();
}

function toRaw(display: string): bigint {
  const [whole, frac = ""] = display.split(".");
  const fracPadded = frac.padEnd(TOKEN_DECIMALS, "0").slice(0, TOKEN_DECIMALS);
  return BigInt(whole || "0") * BigInt(STROOPS) + BigInt(fracPadded || "0");
}

function tsToDate(ts: number | bigint): string {
  return new Date(Number(ts) * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function nowSec(): number {
  return Math.floor(Date.now() / 1000);
}

const VESTING_ERRORS: Record<number, string> = {
  1: "Contract already initialized",
  2: "Contract not yet initialized",
  3: "Unauthorized — admin only",
  4: "Invalid parameter",
  5: "Vesting schedule not found",
  6: "Insufficient token balance",
  7: "Math overflow",
};

function parseContractError(err: unknown): string {
  const msg = String(err);
  const match = msg.match(/Error\(Contract, #(\d+)\)/);
  if (match) return VESTING_ERRORS[Number(match[1])] ?? `Contract error #${match[1]}`;
  if (msg.includes("HostError")) return "Contract execution failed";
  return msg.slice(0, 120);
}

const isValidStellarAddr = (s: string, prefix = "G") =>
  new RegExp(`^${prefix}[A-Z2-7]{55}$`).test(s);

function buildClient(publicKey?: string) {
  return new VestingClient({
    contractId: CONTRACT_ID,
    networkPassphrase: NETWORK_PASSPHRASE,
    rpcUrl: RPC_URL,
    publicKey,
  });
}

async function normaliseSign(
  xdr: string,
  network: string
): Promise<string> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const result = await (signTransaction as any)(xdr, { network });
  if (typeof result === "string") return result;
  if (result?.signedTxXdr) return result.signedTxXdr;
  throw new Error(result?.error ?? "Signing failed");
}

// ─── Role detection ───────────────────────────────────────────────────────────
async function fetchAdmin(): Promise<string | null> {
  // TODO: Replace with actual admin address from your contract
  const HARDCODED_ADMIN = 
    process.env.NEXT_PUBLIC_ADMIN_ADDRESS ??
    "GCRGYF6I7FUTRJIC5RXCUUXISSQK7ZSI47FY6SISPV23JTKBHL2DSNLJ";
  
  console.log("Using hardcoded admin:", HARDCODED_ADMIN); // remove this later
  return HARDCODED_ADMIN;
}

// ─── Types ────────────────────────────────────────────────────────────────────
interface VestingInfo {
  total_amount: bigint;
  claimed: bigint;
  start_time: bigint;
  cliff_time: bigint;
  duration: bigint;
}

type VestingCard = VestingInfo & {
  vesting_id: number;
  claimable: bigint;
};

type Role = "unknown" | "admin" | "user";

// ─── Skeleton ────────────────────────────────────────────────────────────────
function Skeleton({ className = "" }: { className?: string }) {
  return (
    <div
      className={`animate-pulse rounded bg-[#1a1f2e] ${className}`}
      style={{ backgroundImage: "linear-gradient(90deg,#1a1f2e 25%,#252b3b 50%,#1a1f2e 75%)", backgroundSize: "200% 100%", animation: "shimmer 1.5s infinite" }}
    />
  );
}

// ─── Status Badge ────────────────────────────────────────────────────────────
function StatusBadge({ card }: { card: VestingCard }) {
  const now = BigInt(nowSec());
  let label = "Pending";
  let color = "text-amber-400 bg-amber-400/10 border-amber-400/20";
  if (now >= card.cliff_time) {
    if (card.claimed >= card.total_amount) {
      label = "Fully Vested";
      color = "text-emerald-400 bg-emerald-400/10 border-emerald-400/20";
    } else {
      label = "Active";
      color = "text-sky-400 bg-sky-400/10 border-sky-400/20";
    }
  }
  return (
    <span className={`text-xs font-mono px-2 py-0.5 rounded-full border ${color}`}>
      {label}
    </span>
  );
}

// ─── Progress Bar ────────────────────────────────────────────────────────────
function ProgressBar({ value, max }: { value: bigint; max: bigint }) {
  const pct = max > 0n ? Number((value * 100n) / max) : 0;
  return (
    <div className="h-1.5 rounded-full bg-[#1a1f2e] overflow-hidden">
      <div
        className="h-full rounded-full transition-all duration-700"
        style={{ width: `${pct}%`, background: "linear-gradient(90deg, #6366f1, #818cf8)" }}
      />
    </div>
  );
}

// ─── Input field ─────────────────────────────────────────────────────────────
function Field({
  label, value, onChange, placeholder, type = "text", note,
}: {
  label: string; value: string; onChange: (v: string) => void;
  placeholder?: string; type?: string; note?: string;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-mono text-slate-400 uppercase tracking-widest">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="bg-[#0d111c] border border-[#252b3b] rounded-lg px-3 py-2.5 text-sm text-slate-200 font-mono placeholder-slate-600 focus:outline-none focus:border-indigo-500/60 focus:ring-1 focus:ring-indigo-500/30 transition"
      />
      {note && <p className="text-xs text-slate-500">{note}</p>}
    </div>
  );
}

// ─── TxResult ────────────────────────────────────────────────────────────────
function TxResult({ hash, error }: { hash?: string; error?: string }) {
  if (!hash && !error) return null;
  if (error)
    return (
      <div className="mt-3 rounded-lg border border-red-500/20 bg-red-500/5 px-3 py-2 text-xs text-red-400 font-mono break-all">
        ✗ {error}
      </div>
    );
  return (
    <div className="mt-3 rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-3 py-2 text-xs text-emerald-400 font-mono break-all">
      ✓ Tx:{" "}
      <a
        href={`https://stellar.expert/explorer/testnet/tx/${hash}`}
        target="_blank"
        rel="noopener noreferrer"
        className="underline hover:text-emerald-300"
      >
        {hash?.slice(0, 12)}…{hash?.slice(-8)}
      </a>
    </div>
  );
}

// ─── SECTION: Initialize ──────────────────────────────────────────────────────
function InitializeSection({ address }: { address: string }) {
  const [contractId, setContractId] = useState(CONTRACT_ID);
  const [tokenAddr, setTokenAddr] = useState(TOKEN_ADDRESS);
  const [isPending, startTransition] = useTransition();

  const validContract = isValidStellarAddr(contractId, "C");
  const validToken = isValidStellarAddr(tokenAddr, "C");

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const client = buildClient(address);
      const assembled = await client.initialize({
        admin: address,
        token_address: tokenAddr,
      });
      const signed = await normaliseSign(
        assembled.toXDR(),
        "TESTNET"
      );
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = await (assembled as any).signAndSend({ signedTxXdr: signed });
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return { hash: (result as any).getTransactionResponse?.id };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  return (
    <Section title="Initialize Contract" icon="⚙">
      <div className="flex flex-col gap-3">
        <Field label="Contract ID" value={contractId} onChange={setContractId} placeholder="C…" />
        <Field label="Token Address" value={tokenAddr} onChange={setTokenAddr} placeholder="C…" />
        <div className="flex gap-3 text-xs font-mono mt-1">
          <Chip ok={validContract} label="Valid contract ID" />
          <Chip ok={validToken} label="Valid token address" />
        </div>
        <button
          disabled={!validContract || !validToken || isPending}
          onClick={() => startTransition(() => dispatch())}
          className="mt-1 rounded-lg px-4 py-2.5 text-sm font-mono font-semibold bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed transition text-white"
        >
          {isPending ? "Initializing…" : "Initialize"}
        </button>
        <TxResult hash={state?.hash} error={state?.error} />
      </div>
    </Section>
  );
}

function Chip({ ok, label }: { ok: boolean; label: string }) {
  return (
    <span className={`flex items-center gap-1.5 ${ok ? "text-emerald-400" : "text-slate-500"}`}>
      <span>{ok ? "✓" : "○"}</span>{label}
    </span>
  );
}

// ─── SECTION: Create Vesting ──────────────────────────────────────────────────
function CreateVestingSection({ address }: { address: string }) {
  const [beneficiary, setBeneficiary] = useState("");
  const [amount, setAmount] = useState("");
  const [startDt, setStartDt] = useState("");
  const [cliffDt, setCliffDt] = useState("");
  const [durationDays, setDurationDays] = useState("");
  const [isPending, startTransition] = useTransition();

  const validBeneficiary = isValidStellarAddr(beneficiary, "G");
  const canSubmit = validBeneficiary && amount && startDt && cliffDt && durationDays && !isPending;

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const startTs = BigInt(Math.floor(new Date(startDt).getTime() / 1000));
      const cliffTs = BigInt(Math.floor(new Date(cliffDt).getTime() / 1000));
      const durationSecs = BigInt(Math.floor(Number(durationDays) * 86400));
      const rawAmount = toRaw(amount);

      // First approve token spend
      // (Token approval must be done separately via token contract if needed)
      const client = buildClient(address);
      const assembled = await client.create_vesting({
        beneficiary,
        total_amount: rawAmount,
        start_time: startTs,
        cliff_time: cliffTs,
        duration: durationSecs,
      });
      const signed = await normaliseSign(assembled.toXDR(), "TESTNET");
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = await (assembled as any).signAndSend({ signedTxXdr: signed });
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return { hash: (result as any).getTransactionResponse?.id };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  return (
    <Section title="Create Vesting Schedule" icon="＋">
      <div className="flex flex-col gap-3">
        <Field label="Beneficiary Address" value={beneficiary} onChange={setBeneficiary} placeholder="G…" />
        <Field label="Total Amount" value={amount} onChange={setAmount} placeholder="100.0" type="text" note="Token units (7 decimals)" />
        <div className="grid grid-cols-2 gap-3">
          <Field label="Start Time" value={startDt} onChange={setStartDt} type="datetime-local" />
          <Field label="Cliff Time" value={cliffDt} onChange={setCliffDt} type="datetime-local" />
        </div>
        <Field label="Duration (days)" value={durationDays} onChange={setDurationDays} placeholder="365" type="number" />
        <div className="text-xs text-slate-500 font-mono">
          Note: Ensure the token contract has approved this contract to spend your tokens first.
        </div>
        <button
          disabled={!canSubmit}
          onClick={() => startTransition(() => dispatch())}
          className="rounded-lg px-4 py-2.5 text-sm font-mono font-semibold bg-violet-600 hover:bg-violet-500 disabled:opacity-40 disabled:cursor-not-allowed transition text-white"
        >
          {isPending ? "Creating…" : "Create Vesting"}
        </button>
        <TxResult hash={state?.hash} error={state?.error} />
      </div>
    </Section>
  );
}

// ─── SECTION: Emergency Withdraw ──────────────────────────────────────────────
function EmergencyWithdrawSection({ address }: { address: string }) {
  const [amount, setAmount] = useState("");
  const [isPending, startTransition] = useTransition();

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const client = buildClient(address);
      const assembled = await client.emergency_withdraw({ amount: toRaw(amount) });
      const signed = await normaliseSign(assembled.toXDR(), "TESTNET");
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = await (assembled as any).signAndSend({ signedTxXdr: signed });
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return { hash: (result as any).getTransactionResponse?.id };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  return (
    <Section title="Emergency Withdraw" icon="⚠">
      <div className="flex flex-col gap-3">
        <Field label="Amount" value={amount} onChange={setAmount} placeholder="0.0" type="text" note="Amount to withdraw back to admin" />
        <button
          disabled={!amount || isPending}
          onClick={() => startTransition(() => dispatch())}
          className="rounded-lg px-4 py-2.5 text-sm font-mono font-semibold bg-red-700 hover:bg-red-600 disabled:opacity-40 disabled:cursor-not-allowed transition text-white"
        >
          {isPending ? "Withdrawing…" : "Emergency Withdraw"}
        </button>
        <TxResult hash={state?.hash} error={state?.error} />
      </div>
    </Section>
  );
}

// ─── SECTION: Lookup Beneficiary ──────────────────────────────────────────────
function LookupSection() {
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [schedules, setSchedules] = useState<VestingCard[] | null>(null);
  const [lookupError, setLookupError] = useState("");

  const handleLookup = async () => {
    if (!isValidStellarAddr(query, "G")) return;
    setLoading(true);
    setLookupError("");
    try {
      const cards = await fetchVestingSchedules(query);
      setSchedules(cards);
    } catch (e) {
      setLookupError(parseContractError(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Section title="Lookup Beneficiary" icon="🔍">
      <div className="flex gap-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="G… beneficiary address"
          className="flex-1 bg-[#0d111c] border border-[#252b3b] rounded-lg px-3 py-2.5 text-sm text-slate-200 font-mono placeholder-slate-600 focus:outline-none focus:border-indigo-500/60 transition"
        />
        <button
          disabled={!isValidStellarAddr(query, "G") || loading}
          onClick={handleLookup}
          className="rounded-lg px-4 py-2.5 text-sm font-mono font-semibold bg-slate-700 hover:bg-slate-600 disabled:opacity-40 disabled:cursor-not-allowed transition text-white"
        >
          {loading ? "…" : "Lookup"}
        </button>
      </div>
      {lookupError && <p className="text-xs text-red-400 font-mono mt-2">{lookupError}</p>}
      {schedules !== null && (
        <div className="mt-3 flex flex-col gap-3">
          {schedules.length === 0 ? (
            <p className="text-xs text-slate-500 font-mono">No schedules found.</p>
          ) : (
            schedules.map((card) => (
              <VestingCardView key={card.vesting_id} card={card} showClaim={false} address="" />
            ))
          )}
        </div>
      )}
    </Section>
  );
}

// ─── Admin Dashboard ──────────────────────────────────────────────────────────
function AdminDashboard({ address }: { address: string }) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-1">
        <span className="text-xs font-mono px-2 py-0.5 rounded-full border border-violet-400/30 bg-violet-400/10 text-violet-300">
          Admin
        </span>
        <span className="text-xs font-mono text-slate-500 truncate max-w-[220px]">{address}</span>
      </div>
      <InitializeSection address={address} />
      <CreateVestingSection address={address} />
      <EmergencyWithdrawSection address={address} />
      <LookupSection />
    </div>
  );
}

// ─── Fetch vesting schedules ──────────────────────────────────────────────────
async function fetchVestingSchedules(address: string): Promise<VestingCard[]> {
  const client = buildClient();
  const cards: VestingCard[] = [];
  let i = 0;
  while (i < 50) {
    try {
      const tx = await client.get_vesting({ beneficiary: address, index: i });
      // simulate to get return value
      // info would be parsed from ScVal if needed

      // Fallback: use simulateTransaction approach
      const server = new SorobanRpc.Server(RPC_URL);
      const account = await server.getAccount(address).catch(() => null);
      if (!account) break;

      const txToSim = (tx as any).toXDR?.() ?? tx;
      const simResult = await server.simulateTransaction(txToSim);

      if (!("result" in simResult) || !simResult.result) break;

      // Try to decode VestingInfo from ScVal
      const retval = simResult.result.retval;
      if (retval.switch() === StellarSdk.xdr.ScValType.scvVoid()) break;
      if (retval.switch() === StellarSdk.xdr.ScValType.scvMap()) {
        const map = retval.map() ?? [];
        const get = (k: string) =>
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          map.find((e: any) => e.key().sym?.().toString() === k)?.val();

        const vestInfo: VestingInfo = {
          total_amount: BigInt(get("total_amount")?.i128?.().lo?.().toString() ?? 0),
          claimed: BigInt(get("claimed")?.i128?.().lo?.().toString() ?? 0),
          start_time: BigInt(get("start_time")?.u64?.().toString() ?? 0),
          cliff_time: BigInt(get("cliff_time")?.u64?.().toString() ?? 0),
          duration: BigInt(get("duration")?.u64?.().toString() ?? 0),
        };

        // get claimable
        let claimable = 0n;
        try {
          const claimTx = await client.get_claimable_amount({
            beneficiary: address,
            vesting_id: i,
          });
          const claimTxToSim = (claimTx as any).toXDR?.() ?? claimTx;
          const claimSim = await server.simulateTransaction(claimTxToSim);
          if ("result" in claimSim && claimSim.result) {
            claimable = BigInt(claimSim.result.retval.i128?.().lo?.().toString() ?? 0);
          }
        } catch {
          /* ignore */
        }

        cards.push({ ...vestInfo, vesting_id: i, claimable });
        i++;
      } else {
        break;
      }
    } catch {
      break;
    }
  }
  return cards;
}

// ─── Vesting Card ─────────────────────────────────────────────────────────────
function VestingCardView({
  card,
  showClaim,
  address,
  onClaimed,
}: {
  card: VestingCard;
  showClaim: boolean;
  address: string;
  onClaimed?: (id: number) => void;
}) {
  const [optimisticClaimed, addOptimistic] = useOptimistic(
    card.claimed,
    (_state, newClaimed: bigint) => newClaimed
  );
  const [isPending, startTransition] = useTransition();

  type S = { hash?: string; error?: string } | null;
  const [claimState, claimDispatch] = useActionState<S, void>(async () => {
    try {
      const client = buildClient(address);
      const assembled = await client.claim({
        beneficiary: address,
        vesting_id: card.vesting_id,
      });
      const signed = await normaliseSign(assembled.toXDR(), "TESTNET");
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = await (assembled as any).signAndSend({ signedTxXdr: signed });
      const newClaimed = optimisticClaimed + card.claimable;
      addOptimistic(newClaimed);
      onClaimed?.(card.vesting_id);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return { hash: (result as any).getTransactionResponse?.id };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  const endTime = card.start_time + card.duration;

  return (
    <div className="rounded-xl border border-[#252b3b] bg-[#0d111c] p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-mono text-slate-400">Schedule #{card.vesting_id}</span>
        <StatusBadge card={{ ...card, claimed: optimisticClaimed }} />
      </div>

      <div className="grid grid-cols-2 gap-3 text-sm">
        <Stat label="Total" value={toDisplay(card.total_amount)} unit="tokens" />
        <Stat label="Claimed" value={toDisplay(optimisticClaimed)} unit="tokens" accent="text-slate-300" />
        <Stat
          label="Claimable Now"
          value={toDisplay(card.claimable)}
          unit="tokens"
          accent="text-indigo-300 font-semibold"
          highlight
        />
        <Stat label="Remaining" value={toDisplay(card.total_amount - optimisticClaimed)} unit="tokens" />
      </div>

      <ProgressBar value={optimisticClaimed} max={card.total_amount} />
      <div className="flex justify-between text-xs font-mono text-slate-500">
        <span>{toDisplay(optimisticClaimed)} / {toDisplay(card.total_amount)}</span>
        <span>{card.total_amount > 0n ? Number((optimisticClaimed * 100n) / card.total_amount) : 0}%</span>
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs font-mono text-slate-500 border-t border-[#1a1f2e] pt-3">
        <span>Cliff: <span className="text-slate-300">{tsToDate(card.cliff_time)}</span></span>
        <span>Ends: <span className="text-slate-300">{tsToDate(endTime)}</span></span>
      </div>

      {showClaim && (
        <>
          <button
            disabled={card.claimable === 0n || isPending}
            onClick={() => startTransition(() => claimDispatch())}
            className="rounded-lg px-4 py-2 text-sm font-mono font-semibold bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed transition text-white w-full"
          >
            {isPending ? "Claiming…" : `Claim ${toDisplay(card.claimable)} tokens`}
          </button>
          <TxResult hash={claimState?.hash} error={claimState?.error} />
        </>
      )}
    </div>
  );
}

function Stat({
  label, value, unit, accent = "text-slate-200", highlight = false,
}: {
  label: string; value: string; unit: string; accent?: string; highlight?: boolean;
}) {
  return (
    <div className={`flex flex-col gap-0.5 rounded-lg p-2 ${highlight ? "bg-indigo-950/30 border border-indigo-500/20" : "bg-[#131825]"}`}>
      <span className="text-xs text-slate-500 font-mono">{label}</span>
      <span className={`text-sm font-mono ${accent}`}>{value}</span>
      <span className="text-xs text-slate-600 font-mono">{unit}</span>
    </div>
  );
}

// ─── User Dashboard ───────────────────────────────────────────────────────────
function UserDashboard({ address }: { address: string }) {
  const [schedules, setSchedules] = useState<VestingCard[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState("");

  const loadSchedules = useCallback(async () => {
    setLoading(true);
    setFetchError("");
    try {
      const cards = await fetchVestingSchedules(address);
      setSchedules(cards);
    } catch (e) {
      setFetchError(parseContractError(e));
    } finally {
      setLoading(false);
    }
  }, [address]);

  useEffect(() => {
    loadSchedules();
  }, [loadSchedules]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-2">
          <span className="text-xs font-mono px-2 py-0.5 rounded-full border border-sky-400/30 bg-sky-400/10 text-sky-300">
            Beneficiary
          </span>
          <span className="text-xs font-mono text-slate-500 truncate max-w-[200px]">{address}</span>
        </div>
        <button
          onClick={loadSchedules}
          disabled={loading}
          className="text-xs font-mono text-slate-400 hover:text-slate-200 transition"
        >
          ↻ Refresh
        </button>
      </div>

      {loading ? (
        <div className="flex flex-col gap-3">
          {[0, 1].map((i) => (
            <div key={i} className="rounded-xl border border-[#252b3b] p-4 flex flex-col gap-3">
              <Skeleton className="h-4 w-24" />
              <div className="grid grid-cols-2 gap-3">
                <Skeleton className="h-16" />
                <Skeleton className="h-16" />
                <Skeleton className="h-16" />
                <Skeleton className="h-16" />
              </div>
              <Skeleton className="h-2" />
              <Skeleton className="h-9" />
            </div>
          ))}
        </div>
      ) : fetchError ? (
        <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-4 text-xs text-red-400 font-mono">
          {fetchError}
        </div>
      ) : !schedules?.length ? (
        <div className="rounded-xl border border-[#252b3b] p-8 text-center text-slate-500 font-mono text-sm">
          No vesting schedules found for this address.
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {schedules.map((card) => (
            <VestingCardView
              key={card.vesting_id}
              card={card}
              showClaim={true}
              address={address}
              onClaimed={() => loadSchedules()}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Section wrapper ──────────────────────────────────────────────────────────
function Section({
  title, icon, children,
}: {
  title: string; icon: string; children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);
  return (
    <div className="rounded-xl border border-[#252b3b] overflow-hidden">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between px-4 py-3 bg-[#0d111c] hover:bg-[#111827] transition text-left"
      >
        <span className="flex items-center gap-2 text-sm font-mono font-semibold text-slate-200">
          <span>{icon}</span> {title}
        </span>
        <span className="text-slate-500 text-xs">{open ? "▲" : "▼"}</span>
      </button>
      {open && <div className="px-4 py-4 bg-[#080c15]">{children}</div>}
    </div>
  );
}

// ─── Connect Wallet ───────────────────────────────────────────────────────────
function ConnectPanel({ onConnect }: { onConnect: (addr: string) => void }) {
  const [status, setStatus] = useState<"idle" | "connecting" | "error">("idle");
  const [error, setError] = useState("");

  const connect = async () => {
    setStatus("connecting");
    setError("");
    try {
      const connected = await isConnected();
      if (!connected) throw new Error("Freighter not installed");
      const allowed = await isAllowed();
      if (!allowed) await requestAccess();
      const addr = await getAddress();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const address = typeof addr === 'string' ? addr : (addr as any)?.address;
      if (!address) throw new Error("Could not get address");
      onConnect(address);
    } catch (e) {
      setError(String(e));
      setStatus("error");
    }
  };

  return (
    <div className="flex flex-col items-center justify-center gap-6 py-12">
      <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-600 to-violet-600 flex items-center justify-center text-2xl shadow-lg shadow-indigo-500/20">
        ◆
      </div>
      <div className="text-center">
        <h2 className="text-xl font-semibold text-slate-100 mb-1">Vesting Protocol</h2>
        <p className="text-sm text-slate-400 font-mono">Stellar Testnet</p>
      </div>
      <button
        onClick={connect}
        disabled={status === "connecting"}
        className="rounded-xl px-6 py-3 text-sm font-mono font-semibold bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 transition text-white shadow-lg shadow-indigo-500/20"
      >
        {status === "connecting" ? "Connecting…" : "Connect Freighter"}
      </button>
      {error && (
        <p className="text-xs text-red-400 font-mono text-center max-w-xs">{error}</p>
      )}
    </div>
  );
}

// ─── Main Page ────────────────────────────────────────────────────────────────
export default function VestingDApp() {
  const [mounted, setMounted] = useState(false);
  const [address, setAddress] = useState<string | null>(null);
  const [role, setRole] = useState<Role>("unknown");
  const [detectingRole, setDetectingRole] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const handleConnect = async (addr: string) => {
    setAddress(addr);
    setDetectingRole(true);
    try {
      const admin = await fetchAdmin();
      console.log("Connected address:", addr);
      console.log("Admin address from contract:", admin);
      if (admin && admin === addr) {
        console.log("Role: admin");
        setRole("admin");
      } else {
        console.log("Role: user");
        setRole("user");
      }
    } catch (e) {
      console.error("Error detecting role:", e);
      setRole("user");
    } finally {
      setDetectingRole(false);
    }
  };

  const handleDisconnect = () => {
    setAddress(null);
    setRole("unknown");
  };

  if (!mounted) return null;

  return (
    <div
      className="min-h-screen bg-[#060a12] text-slate-200"
      style={{
        backgroundImage:
          "radial-gradient(ellipse at 20% 20%, rgba(99,102,241,0.07) 0%, transparent 60%), radial-gradient(ellipse at 80% 80%, rgba(139,92,246,0.05) 0%, transparent 60%)",
      }}
    >
      {/* Header */}
      <header className="border-b border-[#1a1f2e] px-4 py-3 flex items-center justify-between sticky top-0 bg-[#060a12]/90 backdrop-blur z-10">
        <div className="flex items-center gap-2">
          <span className="text-indigo-400 text-lg">◆</span>
          <span className="text-sm font-mono font-semibold text-slate-200">VestingProtocol</span>
          <span className="text-xs font-mono text-slate-500 hidden sm:block">/ Testnet</span>
        </div>
        {address && (
          <div className="flex items-center gap-3">
            <span className="text-xs font-mono text-slate-400 hidden sm:block">
              {address.slice(0, 6)}…{address.slice(-4)}
            </span>
            <button
              onClick={handleDisconnect}
              className="text-xs font-mono text-slate-500 hover:text-slate-300 transition"
            >
              Disconnect
            </button>
          </div>
        )}
      </header>

      {/* Body */}
      <main className="max-w-xl mx-auto px-4 py-8">
        {!address ? (
          <ConnectPanel onConnect={handleConnect} />
        ) : detectingRole ? (
          <div className="flex flex-col items-center gap-4 py-16">
            <div className="w-8 h-8 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin" />
            <p className="text-sm font-mono text-slate-400">Detecting role…</p>
          </div>
        ) : role === "admin" ? (
          <AdminDashboard address={address} />
        ) : (
          <UserDashboard address={address} />
        )}
      </main>

      <style>{`
        @keyframes shimmer {
          0% { background-position: 200% 0; }
          100% { background-position: -200% 0; }
        }
      `}</style>
    </div>
  );
}