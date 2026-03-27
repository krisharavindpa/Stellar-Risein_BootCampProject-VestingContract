import { Buffer } from "buffer";
import { AssembledTransaction, Client as ContractClient, ClientOptions as ContractClientOptions, MethodOptions, Result } from "@stellar/stellar-sdk/contract";
import type { u32, u64, i128, Option } from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";
export declare const networks: {
    readonly testnet: {
        readonly networkPassphrase: "Test SDF Network ; September 2015";
        readonly contractId: "CBOSKGLRKLRDLDBHNWCJXKCCPSOLJY3KX27QUBOKZPIBOWHIIH22KM2A";
    };
};
export type DataKey = {
    tag: "Admin";
    values: void;
} | {
    tag: "Token";
    values: void;
} | {
    tag: "Vesting";
    values: readonly [string, u32];
} | {
    tag: "VestingCount";
    values: readonly [string];
};
export interface VestingInfo {
    claimed: i128;
    cliff_time: u64;
    duration: u64;
    start_time: u64;
    total_amount: i128;
}
export declare const VestingError: {
    1: {
        message: string;
    };
    2: {
        message: string;
    };
    3: {
        message: string;
    };
    4: {
        message: string;
    };
    5: {
        message: string;
    };
    6: {
        message: string;
    };
    7: {
        message: string;
    };
};
export interface Client {
    /**
     * Construct and simulate a claim transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Claim unlocked tokens for a specific vesting schedule.
     */
    claim: ({ beneficiary, vesting_id }: {
        beneficiary: string;
        vesting_id: u32;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>;
    /**
     * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Initialize the contract with an admin and the token to be vested.
     */
    initialize: ({ admin, token_address }: {
        admin: string;
        token_address: string;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>;
    /**
     * Construct and simulate a get_vesting transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     */
    get_vesting: ({ beneficiary, index }: {
        beneficiary: string;
        index: u32;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Option<VestingInfo>>>;
    /**
     * Construct and simulate a create_vesting transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Creates a new vesting schedule and transfers the total_amount from admin to the contract.
     */
    create_vesting: ({ beneficiary, total_amount, start_time, cliff_time, duration }: {
        beneficiary: string;
        total_amount: i128;
        start_time: u64;
        cliff_time: u64;
        duration: u64;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>;
    /**
     * Construct and simulate a emergency_withdraw transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Admin-only: Withdraw tokens from the contract in case of emergency.
     */
    emergency_withdraw: ({ amount }: {
        amount: i128;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>;
    /**
     * Construct and simulate a get_claimable_amount transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
     * Returns the amount of tokens currently available to claim.
     */
    get_claimable_amount: ({ beneficiary, vesting_id }: {
        beneficiary: string;
        vesting_id: u32;
    }, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>;
}
export declare class Client extends ContractClient {
    readonly options: ContractClientOptions;
    static deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions & Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
    }): Promise<AssembledTransaction<T>>;
    constructor(options: ContractClientOptions);
    readonly fromJSON: {
        claim: (json: string) => AssembledTransaction<Result<bigint, import("@stellar/stellar-sdk/contract").ErrorMessage>>;
        initialize: (json: string) => AssembledTransaction<Result<void, import("@stellar/stellar-sdk/contract").ErrorMessage>>;
        get_vesting: (json: string) => AssembledTransaction<Option<VestingInfo>>;
        create_vesting: (json: string) => AssembledTransaction<Result<void, import("@stellar/stellar-sdk/contract").ErrorMessage>>;
        emergency_withdraw: (json: string) => AssembledTransaction<Result<void, import("@stellar/stellar-sdk/contract").ErrorMessage>>;
        get_claimable_amount: (json: string) => AssembledTransaction<Result<bigint, import("@stellar/stellar-sdk/contract").ErrorMessage>>;
    };
}
