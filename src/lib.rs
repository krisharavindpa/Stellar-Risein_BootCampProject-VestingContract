"use client";

import React, {
  useState,
  useEffect,
  useActionState,
  useOptimistic,
  useTransition,
  useCallback,
} from "react";
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
const DEFAULT_CONTRACT_ID =
  "CBOSKGLRKLRDLDBHNWCJXKCCPSOLJY3KX27QUBOKZPIBOWHIIH22KM2A";
const DEFAULT_TOKEN_ADDRESS =
  "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

const CONTRACT_ID =
  process.env.NEXT_PUBLIC_VESTING_CONTRACT_ID ?? DEFAULT_CONTRACT_ID;
const TOKEN_ADDRESS =
  process.env.NEXT_PUBLIC_TOKEN_ADDRESS ?? DEFAULT_TOKEN_ADDRESS;
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
  const fracStr = frac
    .toString()
    .padStart(TOKEN_DECIMALS, "0")
    .replace(/0+$/, "");
  return fracStr ? `${whole}.${fracStr}` : whole.toString();
}

function toRaw(display: string): bigint {
  const trimmed = display.trim();
  if (!trimmed || isNaN(Number(trimmed))) return 0n;
  const [whole, frac = ""] = trimmed.split(".");
  const fracPadded = frac.padEnd(TOKEN_DECIMALS, "0").slice(0, TOKEN_DECIMALS);
  return (
    BigInt(whole || "0") * BigInt(STROOPS) + BigInt(fracPadded || "0")
  );
}

function tsToDate(ts: number | bigint): string {
  return new Date(Number(ts) * 1000).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
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
  if (match)
    return (
      VESTING_ERRORS[Number(match[1])] ?? `Contract error #${match[1]}`
    );
  if (msg.includes("HostError")) return "Contract execution failed";
  // FIX: surface user-rejection clearly instead of a raw Freighter error string
  if (msg.toLowerCase().includes("rejected")) return "Transaction rejected by user";
  return msg.slice(0, 120);
}

const isValidStellarAddr = (s: string, prefix = "G") =>
  new RegExp(`^${prefix}[A-Z2-7]{55}$`).test(s);

async function isContractDeployed(contractId: string): Promise<boolean> {
  if (!isValidStellarAddr(contractId, "C")) return false;
  try {
    const server = new SorobanRpc.Server(RPC_URL);
    await server.getContractWasmByContractId(contractId);
    return true;
  } catch {
    return false;
  }
}

// ─── Build client ─────────────────────────────────────────────────────────────
// FIX: the signTransaction callback now passes BOTH `network` and
// `networkPassphrase` to Freighter. Passing only `networkPassphrase` (the
// old approach) causes some Freighter versions to auto-reject silently,
// which bubbles up as "The user rejected this request."
//
// FIX: removed the separate `normaliseSign` helper and the
// `toTxClient().signAndSend({ signedTxXdr })` pattern that was used
// alongside it. Those two together signed the transaction TWICE — once
// inside the Soroban client (via this callback) and once manually — so
// Freighter was asked to sign twice per action, and it rejected the
// unexpected second prompt.
//
// All call sites now use `assembled.signAndSend()` with no arguments,
// letting the client handle signing end-to-end through this single callback.
function buildClient(contractId = CONTRACT_ID, publicKey?: string) {
  if (!isValidStellarAddr(contractId, "C")) {
    throw new Error(
      "Invalid vesting contract ID: must be a valid Stellar contract ID starting with C"
    );
  }
  return new VestingClient({
    contractId,
    networkPassphrase: NETWORK_PASSPHRASE,
    rpcUrl: RPC_URL,
    publicKey,
    signTransaction: async (xdr: string) => {
      const result = await signTransaction(xdr, {
        network: "TESTNET",
        networkPassphrase: NETWORK_PASSPHRASE,
      });
      // Freighter v1 returns a plain string; v2+ returns { signedTxXdr, error }
      if (typeof result === "string") return result;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const r = result as any;
      if (r?.signedTxXdr) return r.signedTxXdr;
      throw new Error(r?.error ?? "Signing failed or was rejected");
    },
  });
}

// ─── Role detection ───────────────────────────────────────────────────────────
async function fetchAdmin(): Promise<string | null> {
  const HARDCODED_ADMIN =
    process.env.NEXT_PUBLIC_ADMIN_ADDRESS ??
    "GCRGYF6I7FUTRJIC5RXCUUXISSQK7ZSI47FY6SISPV23JTKBHL2DSNLJ";
  return HARDCODED_ADMIN;
}

// ─── Types ────────────────────────────────────────────────────────────────────
// FIX: all numeric fields typed as bigint. The Soroban SDK maps both u64
// and i128 to bigint, but older SDK versions can return a number for small
// u64 values. We coerce explicitly in fetchVestingSchedules to be safe.
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

// ─── Cliff Presets ────────────────────────────────────────────────────────────
type CliffPreset = "none" | "10s" | "1y" | "5y" | "custom";

interface CliffPresetOption {
  id: CliffPreset;
  label: string;
  sublabel: string;
  // null  → custom date picker
  // 0     → no cliff (cliff === start)
  offsetSeconds: number | null;
}

const CLIFF_PRESETS: CliffPresetOption[] = [
  {
    id: "none",
    label: "No Cliff",
    sublabel: "Starts immediately",
    offsetSeconds: 0,
  },
  {
    id: "10s",
    label: "10 Seconds",
    sublabel: "For testing",
    offsetSeconds: 10,
  },
  {
    id: "1y",
    label: "1 Year",
    sublabel: "12-month cliff",
    offsetSeconds: 365 * 24 * 60 * 60,
  },
  {
    id: "5y",
    label: "5 Years",
    sublabel: "60-month cliff",
    offsetSeconds: 5 * 365 * 24 * 60 * 60,
  },
  {
    id: "custom",
    label: "Custom",
    sublabel: "Pick a date & time",
    offsetSeconds: null,
  },
];

// ─── Skeleton ────────────────────────────────────────────────────────────────
function Skeleton({ className = "" }: { className?: string }) {
  return (
    <div
      className={`animate-pulse rounded bg-[#1a1f2e] ${className}`}
      style={{
        backgroundImage:
          "linear-gradient(90deg,#1a1f2e 25%,#252b3b 50%,#1a1f2e 75%)",
        backgroundSize: "200% 100%",
        animation: "shimmer 1.5s infinite",
      }}
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
    <span
      className={`text-xs font-mono px-2 py-0.5 rounded-full border ${color}`}
    >
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
        style={{
          width: `${pct}%`,
          background: "linear-gradient(90deg, #6366f1, #818cf8)",
        }}
      />
    </div>
  );
}

// ─── Input field ─────────────────────────────────────────────────────────────
function Field({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  note,
  min,
  children,
}: {
  label: string;
  value?: string;
  onChange?: (v: string) => void;
  placeholder?: string;
  type?: string;
  note?: string;
  min?: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-mono text-slate-400 uppercase tracking-widest">
        {label}
      </label>
      <div className="relative flex items-center">
        {children ? (
          children
        ) : (
          <input
            type={type}
            value={value}
            onChange={(e) => onChange?.(e.target.value)}
            placeholder={placeholder}
            min={min}
            className="w-full bg-[#0d111c] border border-[#252b3b] rounded-lg px-3 py-2.5 text-sm text-slate-200 font-mono placeholder-slate-600 focus:outline-none focus:border-indigo-500/60 focus:ring-1 focus:ring-indigo-500/30 transition"
          />
        )}
      </div>
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

// ─── Cliff Preset Selector ────────────────────────────────────────────────────
function CliffPresetSelector({
  selected,
  onSelect,
}: {
  selected: CliffPreset;
  onSelect: (preset: CliffPreset) => void;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label className="text-xs font-mono text-slate-400 uppercase tracking-widest">
        Cliff Period
      </label>
      <div className="grid grid-cols-4 gap-2">
        {CLIFF_PRESETS.map((preset) => {
          const isActive = selected === preset.id;
          return (
            <button
              key={preset.id}
              type="button"
              onClick={() => onSelect(preset.id)}
              className={`
                relative flex flex-col items-center justify-center gap-0.5
                rounded-lg border px-2 py-3 text-center transition-all duration-150
                ${
                  isActive
                    ? "border-indigo-500/60 bg-indigo-950/50 ring-1 ring-indigo-500/30"
                    : "border-[#252b3b] bg-[#0d111c] hover:border-[#353d52] hover:bg-[#111827]"
                }
              `}
            >
              {isActive && (
                <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 rounded-full bg-indigo-400" />
              )}
              <span
                className={`text-sm font-mono font-semibold ${
                  isActive ? "text-indigo-300" : "text-slate-200"
                }`}
              >
                {preset.label}
              </span>
              <span className="text-[10px] font-mono text-slate-500 leading-tight text-center">
                {preset.sublabel}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ─── SECTION: Initialize ──────────────────────────────────────────────────────
function InitializeSection({
  address,
  contractId: initialContractId,
  tokenAddress: initialTokenAddress,
  onConfigChange,
}: {
  address: string;
  contractId: string;
  tokenAddress: string;
  onConfigChange: (contractId: string, tokenAddress: string) => void;
}) {
  const [contractId, setContractId] = useState(initialContractId);
  const [tokenAddr, setTokenAddr] = useState(initialTokenAddress);
  const [isPending, startTransition] = useTransition();
  const [contractExists, setContractExists] = useState<boolean | null>(null);

  const validContract = isValidStellarAddr(contractId, "C");
  const validToken = isValidStellarAddr(tokenAddr, "C");

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const client = buildClient(contractId, address);
      const assembled = await client.initialize({
        admin: address,
        token_address: tokenAddr,
      });
      // FIX: use assembled.signAndSend() — no manual normaliseSign needed
      const result = await assembled.signAndSend();
      onConfigChange(contractId, tokenAddr);
      return {
        hash:
          (result as any)?.sendTransactionResponse?.hash ??
          (result as any)?.getTransactionResponse?.id,
      };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  useEffect(() => {
    let canceled = false;
    if (!validContract) return;
    (async () => {
      const exists = await isContractDeployed(contractId);
      if (!canceled) setContractExists(exists);
    })();
    return () => {
      canceled = true;
    };
  }, [contractId, validContract]);

  return (
    <Section title="Initialize Contract" icon="⚙">
      <div className="flex flex-col gap-3">
        <Field
          label="Contract ID"
          value={contractId}
          onChange={(v) => {
            setContractId(v);
            onConfigChange(v, tokenAddr);
          }}
          placeholder="C…"
        />
        <Field
          label="Token Address"
          value={tokenAddr}
          onChange={(v) => {
            setTokenAddr(v);
            onConfigChange(contractId, v);
          }}
          placeholder="C…"
        />
        <div className="flex gap-3 text-xs font-mono mt-1">
          <Chip ok={validContract} label="Valid contract ID" />
          <Chip ok={validToken} label="Valid token address" />
          <Chip
            ok={contractExists === true}
            label={
              contractExists === true
                ? "Contract deployed"
                : contractExists === false
                ? "Contract not found"
                : "Checking contract..."
            }
          />
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
    <span
      className={`flex items-center gap-1.5 ${
        ok ? "text-emerald-400" : "text-slate-500"
      }`}
    >
      <span>{ok ? "✓" : "○"}</span>
      {label}
    </span>
  );
}

// ─── SECTION: Create Vesting ──────────────────────────────────────────────────
function formatDateTimeLocal(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");
  return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}`;
}

function addSecondsToDateString(dateStr: string, seconds: number): string {
  const base = new Date(dateStr);
  if (isNaN(base.getTime())) return "";
  base.setTime(base.getTime() + seconds * 1000);
  return formatDateTimeLocal(base);
}

function CreateVestingSection({
  address,
  contractId,
}: {
  address: string;
  contractId: string;
}) {
  const now = new Date();
  const defaultStart = formatDateTimeLocal(now);

  const [beneficiary, setBeneficiary] = useState("");
  const [amount, setAmount] = useState("");
  const [startDt, setStartDt] = useState(defaultStart);

  const [durationValue, setDurationValue] = useState("30");
  const [durationUnit, setDurationUnit] = useState<"days" | "seconds">("days");

  const [cliffPreset, setCliffPreset] = useState<CliffPreset>("none");
  const [customCliffDt, setCustomCliffDt] = useState(
    formatDateTimeLocal(new Date(now.getTime() + 24 * 60 * 60 * 1000))
  );

  const [isPending, startTransition] = useTransition();

  const effectiveCliffDt: string = (() => {
    const preset = CLIFF_PRESETS.find((p) => p.id === cliffPreset)!;
    if (preset.offsetSeconds === null) return customCliffDt;
    return addSecondsToDateString(startDt, preset.offsetSeconds);
  })();

  const handleStartChange = (v: string) => {
    setStartDt(v);
    if (cliffPreset === "custom") {
      const newStart = new Date(v);
      const existingCliff = new Date(customCliffDt);
      if (!isNaN(newStart.getTime()) && existingCliff < newStart) {
        setCustomCliffDt(formatDateTimeLocal(newStart));
      }
    }
  };

  const handlePresetSelect = (preset: CliffPreset) => {
    if (preset === "custom") {
      const current = effectiveCliffDt;
      setCustomCliffDt(current || addSecondsToDateString(startDt, 86400));
    }
    setCliffPreset(preset);
  };

  const validBeneficiary = isValidStellarAddr(beneficiary, "G");

  const startTs = (() => {
    const d = new Date(startDt);
    return isNaN(d.getTime()) ? 0 : Math.floor(d.getTime() / 1000);
  })();
  const cliffTs = (() => {
    const d = new Date(effectiveCliffDt);
    return isNaN(d.getTime()) ? 0 : Math.floor(d.getTime() / 1000);
  })();

  const durationValNum = Number(durationValue);
  const durationSecs =
    !isNaN(durationValNum) && durationValNum > 0
      ? Math.floor(durationValNum * (durationUnit === "days" ? 86400 : 1))
      : 0;

  const validDates = startTs > 0 && cliffTs > 0;
  const cliffNotBeforeStart = cliffTs >= startTs;
  const cliffWithinRange =
    durationSecs > 0 && cliffTs <= startTs + durationSecs;
  const validTimes = validDates && cliffNotBeforeStart && durationSecs > 0;

  const properlySizedAmount = (() => {
    const checked = Number(amount);
    return !Number.isNaN(checked) && checked > 0;
  })();

  const canSubmit =
    validBeneficiary &&
    properlySizedAmount &&
    validTimes &&
    cliffWithinRange &&
    !isPending;

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const startTsBig = BigInt(startTs);
      const cliffTsBig = BigInt(cliffTs);
      const durationSecsBig = BigInt(Math.floor(durationSecs));
      const rawAmount = toRaw(amount);

      if (rawAmount === 0n) throw new Error("Amount must be greater than zero");

      const client = buildClient(contractId, address);
      const assembled = await client.create_vesting({
        beneficiary,
        total_amount: rawAmount,
        start_time: startTsBig,
        cliff_time: cliffTsBig,
        duration: durationSecsBig,
      });

      // FIX: use assembled.signAndSend() — signing is handled by buildClient's
      // callback. The previous pattern called normaliseSign (sign #1) then
      // toTxClient().signAndSend({ signedTxXdr }) (sign #2), triggering two
      // Freighter prompts. Freighter rejected the second one, which appeared
      // to the user as "The user rejected this request."
      const result = await assembled.signAndSend();
      return {
        hash:
          (result as any)?.sendTransactionResponse?.hash ??
          (result as any)?.getTransactionResponse?.id,
      };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  const cliffSummary = (() => {
    if (cliffPreset === "none")
      return "No cliff — vesting begins immediately at start time";
    if (!cliffTs) return "Set a valid start time to compute cliff date";
    const dateLabel = tsToDate(cliffTs);
    if (cliffPreset === "custom")
      return `Custom cliff — tokens unlock from ${dateLabel}`;
    const p = CLIFF_PRESETS.find((x) => x.id === cliffPreset)!;
    return `${p.label} cliff — tokens unlock from ${dateLabel}`;
  })();

  return (
    <Section title="Create Vesting Schedule" icon="＋">
      <div className="flex flex-col gap-3">
        <Field
          label="Beneficiary Address"
          value={beneficiary}
          onChange={setBeneficiary}
          placeholder="G…"
        />
        <Field
          label="Total Amount"
          value={amount}
          onChange={setAmount}
          placeholder="100.0"
          type="text"
          note="Token units (7 decimals)"
        />

        <div className="grid grid-cols-2 gap-3">
          <Field
            label="Start Time"
            value={startDt}
            onChange={handleStartChange}
            type="datetime-local"
            min={formatDateTimeLocal(new Date())}
          />

          <div className="flex flex-col gap-1">
            <label className="text-xs font-mono text-slate-400 uppercase tracking-widest">
              Duration
            </label>
            <div className="flex bg-[#0d111c] border border-[#252b3b] rounded-lg overflow-hidden focus-within:border-indigo-500/60 focus-within:ring-1 focus-within:ring-indigo-500/30 transition">
              <input
                type="number"
                value={durationValue}
                onChange={(e) => setDurationValue(e.target.value)}
                placeholder="30"
                min="1"
                className="w-full bg-transparent px-3 py-2.5 text-sm text-slate-200 font-mono placeholder-slate-600 focus:outline-none"
              />
              <select
                value={durationUnit}
                onChange={(e) =>
                  setDurationUnit(e.target.value as "days" | "seconds")
                }
                className="bg-[#1a1f2e] text-xs font-mono text-slate-300 border-l border-[#252b3b] px-2 outline-none cursor-pointer hover:bg-[#252b3b]"
              >
                <option value="days">Days</option>
                <option value="seconds">Secs</option>
              </select>
            </div>
          </div>
        </div>

        <CliffPresetSelector
          selected={cliffPreset}
          onSelect={handlePresetSelect}
        />

        {cliffPreset === "custom" && (
          <Field
            label="Custom Cliff Date & Time"
            value={customCliffDt}
            onChange={setCustomCliffDt}
            type="datetime-local"
            min={startDt}
          />
        )}

        <div className="flex items-start gap-2 rounded-lg border border-indigo-500/20 bg-indigo-950/20 px-3 py-2.5">
          <span className="text-indigo-400 mt-0.5 text-xs shrink-0">◆</span>
          <span className="text-xs font-mono text-indigo-300/80">
            {cliffSummary}
          </span>
        </div>

        {!cliffNotBeforeStart && cliffTs > 0 && (
          <p className="text-xs text-red-400 font-mono">
            ✗ Cliff time cannot be before start time.
          </p>
        )}
        {cliffNotBeforeStart && !cliffWithinRange && durationSecs > 0 && (
          <p className="text-xs text-red-400 font-mono">
            ✗ Cliff must be within the vesting duration (≤ start +{" "}
            {durationSecs}s).
          </p>
        )}
        {durationSecs === 0 && durationValue !== "" && (
          <p className="text-xs text-red-400 font-mono">
            ✗ Duration must be greater than zero.
          </p>
        )}

        <div className="text-xs text-slate-500 font-mono">
          Note: Ensure the token contract has approved this contract to spend
          your tokens first.
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
function EmergencyWithdrawSection({
  address,
  contractId,
}: {
  address: string;
  contractId: string;
}) {
  const [amount, setAmount] = useState("");
  const [isPending, startTransition] = useTransition();

  type S = { hash?: string; error?: string } | null;
  const [state, dispatch] = useActionState<S, void>(async () => {
    try {
      const rawAmount = toRaw(amount);
      if (rawAmount === 0n) throw new Error("Amount must be greater than zero");
      const client = buildClient(contractId, address);
      // FIX: same double-sign fix — use assembled.signAndSend() only
      const assembled = await client.emergency_withdraw({ amount: rawAmount });
      const result = await assembled.signAndSend();
      return {
        hash:
          (result as any)?.sendTransactionResponse?.hash ??
          (result as any)?.getTransactionResponse?.id,
      };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  return (
    <Section title="Emergency Withdraw" icon="⚠">
      <div className="flex flex-col gap-3">
        <Field
          label="Amount"
          value={amount}
          onChange={setAmount}
          placeholder="0.0"
          type="text"
          note="Amount to withdraw back to admin"
        />
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
function LookupSection({ contractId }: { contractId: string }) {
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [schedules, setSchedules] = useState<VestingCard[] | null>(null);
  const [lookupError, setLookupError] = useState("");

  const handleLookup = async () => {
    if (!isValidStellarAddr(query, "G")) return;
    setLoading(true);
    setLookupError("");
    try {
      const cards = await fetchVestingSchedules(query, contractId);
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
      {lookupError && (
        <p className="text-xs text-red-400 font-mono mt-2">{lookupError}</p>
      )}
      {schedules !== null && (
        <div className="mt-3 flex flex-col gap-3">
          {schedules.length === 0 ? (
            <p className="text-xs text-slate-500 font-mono">
              No schedules found.
            </p>
          ) : (
            schedules.map((card) => (
              <VestingCardView
                key={card.vesting_id}
                card={card}
                showClaim={false}
                address=""
                contractId={contractId}
              />
            ))
          )}
        </div>
      )}
    </Section>
  );
}

// ─── Admin Dashboard ──────────────────────────────────────────────────────────
function AdminDashboard({
  address,
  contractId,
  tokenAddress,
  onConfigChange,
}: {
  address: string;
  contractId: string;
  tokenAddress: string;
  onConfigChange: (contractId: string, tokenAddress: string) => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-1">
        <span className="text-xs font-mono px-2 py-0.5 rounded-full border border-violet-400/30 bg-violet-400/10 text-violet-300">
          Admin
        </span>
        <span className="text-xs font-mono text-slate-500 truncate max-w-[220px]">
          {address}
        </span>
      </div>
      <InitializeSection
        address={address}
        contractId={contractId}
        tokenAddress={tokenAddress}
        onConfigChange={onConfigChange}
      />
      <CreateVestingSection address={address} contractId={contractId} />
      <EmergencyWithdrawSection address={address} contractId={contractId} />
      <LookupSection contractId={contractId} />
    </div>
  );
}

// ─── Fetch vesting schedules ──────────────────────────────────────────────────
async function fetchVestingSchedules(
  address: string,
  contractId: string
): Promise<VestingCard[]> {
  // Read-only client — no publicKey or signTransaction needed
  const client = buildClient(contractId);
  const cards: VestingCard[] = [];
  let i = 0;
  while (i < 50) {
    try {
      const tx = await client.get_vesting({ beneficiary: address, index: i });
      const vestingData = tx.result;
      if (!vestingData) break;

      // FIX: explicitly coerce all fields to bigint.
      // Older Soroban SDK versions may return number for small u64 values,
      // which would silently break all bigint arithmetic in the UI.
      const vestInfo: VestingInfo = {
        total_amount: BigInt(vestingData.total_amount),
        claimed: BigInt(vestingData.claimed),
        start_time: BigInt(vestingData.start_time),
        cliff_time: BigInt(vestingData.cliff_time),
        duration: BigInt(vestingData.duration),
      };

      let claimable = 0n;
      try {
        const claimTx = await client.get_claimable_amount({
          beneficiary: address,
          vesting_id: i,
        });
        // FIX: guard against undefined/null before coercing
        if (claimTx.result !== undefined && claimTx.result !== null) {
          claimable = BigInt(claimTx.result);
        }
      } catch {
        /* non-fatal: claimable stays 0n */
      }

      cards.push({ ...vestInfo, vesting_id: i, claimable });
      i++;
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
  contractId,
  onClaimed,
}: {
  card: VestingCard;
  showClaim: boolean;
  address: string;
  contractId: string;
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
      const client = buildClient(contractId, address);
      const assembled = await client.claim({
        beneficiary: address,
        vesting_id: card.vesting_id,
      });
      // FIX: same double-sign fix — use assembled.signAndSend() only
      const result = await assembled.signAndSend();
      const newClaimed = optimisticClaimed + card.claimable;
      addOptimistic(newClaimed);
      onClaimed?.(card.vesting_id);
      return {
        hash:
          (result as any)?.sendTransactionResponse?.hash ??
          (result as any)?.getTransactionResponse?.id,
      };
    } catch (e) {
      return { error: parseContractError(e) };
    }
  }, null);

  const endTime = card.start_time + card.duration;

  return (
    <div className="rounded-xl border border-[#252b3b] bg-[#0d111c] p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-mono text-slate-400">
          Schedule #{card.vesting_id}
        </span>
        <StatusBadge card={{ ...card, claimed: optimisticClaimed }} />
      </div>

      <div className="grid grid-cols-2 gap-3 text-sm">
        <Stat label="Total" value={toDisplay(card.total_amount)} unit="tokens" />
        <Stat
          label="Claimed"
          value={toDisplay(optimisticClaimed)}
          unit="tokens"
          accent="text-slate-300"
        />
        <Stat
          label="Claimable Now"
          value={toDisplay(card.claimable)}
          unit="tokens"
          accent="text-indigo-300 font-semibold"
          highlight
        />
        <Stat
          label="Remaining"
          value={toDisplay(card.total_amount - optimisticClaimed)}
          unit="tokens"
        />
      </div>

      <ProgressBar value={optimisticClaimed} max={card.total_amount} />
      <div className="flex justify-between text-xs font-mono text-slate-500">
        <span>
          {toDisplay(optimisticClaimed)} / {toDisplay(card.total_amount)}
        </span>
        <span>
          {card.total_amount > 0n
            ? Number((optimisticClaimed * 100n) / card.total_amount)
            : 0}
          %
        </span>
      </div>

      <div className="grid grid-cols-1 gap-1 text-xs font-mono text-slate-500 border-t border-[#1a1f2e] pt-3">
        <span>
          Cliff:{" "}
          <span className="text-slate-300">{tsToDate(card.cliff_time)}</span>
        </span>
        <span>
          Ends: <span className="text-slate-300">{tsToDate(endTime)}</span>
        </span>
      </div>

      {showClaim && (
        <>
          <button
            disabled={card.claimable === 0n || isPending}
            onClick={() => startTransition(() => claimDispatch())}
            className="rounded-lg px-4 py-2 text-sm font-mono font-semibold bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed transition text-white w-full"
          >
            {isPending
              ? "Claiming…"
              : `Claim ${toDisplay(card.claimable)} tokens`}
          </button>
          <TxResult hash={claimState?.hash} error={claimState?.error} />
        </>
      )}
    </div>
  );
}

function Stat({
  label,
  value,
  unit,
  accent = "text-slate-200",
  highlight = false,
}: {
  label: string;
  value: string;
  unit: string;
  accent?: string;
  highlight?: boolean;
}) {
  return (
    <div
      className={`flex flex-col gap-0.5 rounded-lg p-2 ${
        highlight
          ? "bg-indigo-950/30 border border-indigo-500/20"
          : "bg-[#131825]"
      }`}
    >
      <span className="text-xs text-slate-500 font-mono">{label}</span>
      <span className={`text-sm font-mono ${accent}`}>{value}</span>
      <span className="text-xs text-slate-600 font-mono">{unit}</span>
    </div>
  );
}

// ─── User Dashboard ───────────────────────────────────────────────────────────
function UserDashboard({
  address,
  contractId,
}: {
  address: string;
  contractId: string;
}) {
  const [schedules, setSchedules] = useState<VestingCard[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState("");

  const loadSchedules = useCallback(async () => {
    setLoading(true);
    setFetchError("");
    try {
      const cards = await fetchVestingSchedules(address, contractId);
      setSchedules(cards);
    } catch (e) {
      setFetchError(parseContractError(e));
    } finally {
      setLoading(false);
    }
  }, [address, contractId]);

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
          <span className="text-xs font-mono text-slate-500 truncate max-w-[200px]">
            {address}
          </span>
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
            <div
              key={i}
              className="rounded-xl border border-[#252b3b] p-4 flex flex-col gap-3"
            >
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
              contractId={contractId}
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
  title,
  icon,
  children,
}: {
  title: string;
  icon: string;
  children: React.ReactNode;
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
      const address = typeof addr === "string" ? addr : (addr as any)?.address;
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
        <h2 className="text-xl font-semibold text-slate-100 mb-1">
          Vesting Protocol
        </h2>
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
        <p className="text-xs text-red-400 font-mono text-center max-w-xs">
          {error}
        </p>
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

  const [effectiveContractId, setEffectiveContractId] = useState(CONTRACT_ID);
  const [effectiveTokenAddress, setEffectiveTokenAddress] =
    useState(TOKEN_ADDRESS);

  useEffect(() => {
    setMounted(true);
  }, []);

  const handleConnect = async (addr: string) => {
    setAddress(addr);
    setDetectingRole(true);
    try {
      const admin = await fetchAdmin();
      if (admin && admin === addr) {
        setRole("admin");
      } else {
        setRole("user");
      }
    } catch {
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
      <header className="border-b border-[#1a1f2e] px-4 py-3 flex items-center justify-between sticky top-0 bg-[#060a12]/90 backdrop-blur z-10">
        <div className="flex items-center gap-2">
          <span className="text-indigo-400 text-lg">◆</span>
          <span className="text-sm font-mono font-semibold text-slate-200">
            VestingProtocol
          </span>
          <span className="text-xs font-mono text-slate-500 hidden sm:block">
            / Testnet
          </span>
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

      <main className="max-w-xl mx-auto px-4 py-8">
        {!address ? (
          <ConnectPanel onConnect={handleConnect} />
        ) : detectingRole ? (
          <div className="flex flex-col items-center gap-4 py-16">
            <div className="w-8 h-8 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin" />
            <p className="text-sm font-mono text-slate-400">Detecting role…</p>
          </div>
        ) : role === "admin" ? (
          <AdminDashboard
            address={address}
            contractId={effectiveContractId}
            tokenAddress={effectiveTokenAddress}
            onConfigChange={(c, t) => {
              setEffectiveContractId(c);
              setEffectiveTokenAddress(t);
            }}
          />
        ) : (
          <UserDashboard
            address={address}
            contractId={effectiveContractId}
          />
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