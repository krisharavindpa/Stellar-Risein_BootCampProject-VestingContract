import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}


export const networks = {
  testnet: {
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CBOSKGLRKLRDLDBHNWCJXKCCPSOLJY3KX27QUBOKZPIBOWHIIH22KM2A",
  }
} as const

export type DataKey = {tag: "Admin", values: void} | {tag: "Token", values: void} | {tag: "Vesting", values: readonly [string, u32]} | {tag: "VestingCount", values: readonly [string]};


export interface VestingInfo {
  claimed: i128;
  cliff_time: u64;
  duration: u64;
  start_time: u64;
  total_amount: i128;
}

export const VestingError = {
  1: {message:"AlreadyInitialized"},
  2: {message:"NotInitialized"},
  3: {message:"Unauthorized"},
  4: {message:"InvalidParam"},
  5: {message:"NotFound"},
  6: {message:"InsufficientBalance"},
  7: {message:"MathOverflow"}
}

export interface Client {
  /**
   * Construct and simulate a claim transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Claim unlocked tokens for a specific vesting schedule.
   */
  claim: ({beneficiary, vesting_id}: {beneficiary: string, vesting_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Initialize the contract with an admin and the token to be vested.
   */
  initialize: ({admin, token_address}: {admin: string, token_address: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_vesting transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_vesting: ({beneficiary, index}: {beneficiary: string, index: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Option<VestingInfo>>>

  /**
   * Construct and simulate a create_vesting transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Creates a new vesting schedule and transfers the total_amount from admin to the contract.
   */
  create_vesting: ({beneficiary, total_amount, start_time, cliff_time, duration}: {beneficiary: string, total_amount: i128, start_time: u64, cliff_time: u64, duration: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a emergency_withdraw transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Admin-only: Withdraw tokens from the contract in case of emergency.
   */
  emergency_withdraw: ({amount}: {amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_claimable_amount transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the amount of tokens currently available to claim.
   */
  get_claimable_amount: ({beneficiary, vesting_id}: {beneficiary: string, vesting_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAAAAAAADZDbGFpbSB1bmxvY2tlZCB0b2tlbnMgZm9yIGEgc3BlY2lmaWMgdmVzdGluZyBzY2hlZHVsZS4AAAAAAAVjbGFpbQAAAAAAAAIAAAAAAAAAC2JlbmVmaWNpYXJ5AAAAABMAAAAAAAAACnZlc3RpbmdfaWQAAAAAAAQAAAABAAAD6QAAAAsAAAfQAAAADFZlc3RpbmdFcnJvcg==",
        "AAAAAgAAAAAAAAAAAAAAB0RhdGFLZXkAAAAABAAAAAAAAAAAAAAABUFkbWluAAAAAAAAAAAAAAAAAAAFVG9rZW4AAAAAAAABAAAAAAAAAAdWZXN0aW5nAAAAAAIAAAATAAAABAAAAAEAAAAAAAAADFZlc3RpbmdDb3VudAAAAAEAAAAT",
        "AAAAAAAAAEFJbml0aWFsaXplIHRoZSBjb250cmFjdCB3aXRoIGFuIGFkbWluIGFuZCB0aGUgdG9rZW4gdG8gYmUgdmVzdGVkLgAAAAAAAAppbml0aWFsaXplAAAAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAADXRva2VuX2FkZHJlc3MAAAAAAAATAAAAAQAAA+kAAAPtAAAAAAAAB9AAAAAMVmVzdGluZ0Vycm9y",
        "AAAAAAAAAAAAAAALZ2V0X3Zlc3RpbmcAAAAAAgAAAAAAAAALYmVuZWZpY2lhcnkAAAAAEwAAAAAAAAAFaW5kZXgAAAAAAAAEAAAAAQAAA+gAAAfQAAAAC1Zlc3RpbmdJbmZvAA==",
        "AAAAAQAAAAAAAAAAAAAAC1Zlc3RpbmdJbmZvAAAAAAUAAAAAAAAAB2NsYWltZWQAAAAACwAAAAAAAAAKY2xpZmZfdGltZQAAAAAABgAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAAAAAApzdGFydF90aW1lAAAAAAAGAAAAAAAAAAx0b3RhbF9hbW91bnQAAAAL",
        "AAAAAAAAAFlDcmVhdGVzIGEgbmV3IHZlc3Rpbmcgc2NoZWR1bGUgYW5kIHRyYW5zZmVycyB0aGUgdG90YWxfYW1vdW50IGZyb20gYWRtaW4gdG8gdGhlIGNvbnRyYWN0LgAAAAAAAA5jcmVhdGVfdmVzdGluZwAAAAAABQAAAAAAAAALYmVuZWZpY2lhcnkAAAAAEwAAAAAAAAAMdG90YWxfYW1vdW50AAAACwAAAAAAAAAKc3RhcnRfdGltZQAAAAAABgAAAAAAAAAKY2xpZmZfdGltZQAAAAAABgAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAQAAA+kAAAPtAAAAAAAAB9AAAAAMVmVzdGluZ0Vycm9y",
        "AAAABAAAAAAAAAAAAAAADFZlc3RpbmdFcnJvcgAAAAcAAAAAAAAAEkFscmVhZHlJbml0aWFsaXplZAAAAAAAAQAAAAAAAAAOTm90SW5pdGlhbGl6ZWQAAAAAAAIAAAAAAAAADFVuYXV0aG9yaXplZAAAAAMAAAAAAAAADEludmFsaWRQYXJhbQAAAAQAAAAAAAAACE5vdEZvdW5kAAAABQAAAAAAAAATSW5zdWZmaWNpZW50QmFsYW5jZQAAAAAGAAAAAAAAAAxNYXRoT3ZlcmZsb3cAAAAH",
        "AAAAAAAAAENBZG1pbi1vbmx5OiBXaXRoZHJhdyB0b2tlbnMgZnJvbSB0aGUgY29udHJhY3QgaW4gY2FzZSBvZiBlbWVyZ2VuY3kuAAAAABJlbWVyZ2VuY3lfd2l0aGRyYXcAAAAAAAEAAAAAAAAABmFtb3VudAAAAAAACwAAAAEAAAPpAAAD7QAAAAAAAAfQAAAADFZlc3RpbmdFcnJvcg==",
        "AAAAAAAAADpSZXR1cm5zIHRoZSBhbW91bnQgb2YgdG9rZW5zIGN1cnJlbnRseSBhdmFpbGFibGUgdG8gY2xhaW0uAAAAAAAUZ2V0X2NsYWltYWJsZV9hbW91bnQAAAACAAAAAAAAAAtiZW5lZmljaWFyeQAAAAATAAAAAAAAAAp2ZXN0aW5nX2lkAAAAAAAEAAAAAQAAA+kAAAALAAAH0AAAAAxWZXN0aW5nRXJyb3I=" ]),
      options
    )
  }
  public readonly fromJSON = {
    claim: this.txFromJSON<Result<i128>>,
        initialize: this.txFromJSON<Result<void>>,
        get_vesting: this.txFromJSON<Option<VestingInfo>>,
        create_vesting: this.txFromJSON<Result<void>>,
        emergency_withdraw: this.txFromJSON<Result<void>>,
        get_claimable_amount: this.txFromJSON<Result<i128>>
  }
}