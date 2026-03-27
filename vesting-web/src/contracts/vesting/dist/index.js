import { Buffer } from "buffer";
import { Client as ContractClient, Spec as ContractSpec, } from "@stellar/stellar-sdk/contract";
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
};
export const VestingError = {
    1: { message: "AlreadyInitialized" },
    2: { message: "NotInitialized" },
    3: { message: "Unauthorized" },
    4: { message: "InvalidParam" },
    5: { message: "NotFound" },
    6: { message: "InsufficientBalance" },
    7: { message: "MathOverflow" }
};
export class Client extends ContractClient {
    options;
    static async deploy(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options) {
        return ContractClient.deploy(null, options);
    }
    constructor(options) {
        super(new ContractSpec(["AAAAAAAAADZDbGFpbSB1bmxvY2tlZCB0b2tlbnMgZm9yIGEgc3BlY2lmaWMgdmVzdGluZyBzY2hlZHVsZS4AAAAAAAVjbGFpbQAAAAAAAAIAAAAAAAAAC2JlbmVmaWNpYXJ5AAAAABMAAAAAAAAACnZlc3RpbmdfaWQAAAAAAAQAAAABAAAD6QAAAAsAAAfQAAAADFZlc3RpbmdFcnJvcg==",
            "AAAAAgAAAAAAAAAAAAAAB0RhdGFLZXkAAAAABAAAAAAAAAAAAAAABUFkbWluAAAAAAAAAAAAAAAAAAAFVG9rZW4AAAAAAAABAAAAAAAAAAdWZXN0aW5nAAAAAAIAAAATAAAABAAAAAEAAAAAAAAADFZlc3RpbmdDb3VudAAAAAEAAAAT",
            "AAAAAAAAAEFJbml0aWFsaXplIHRoZSBjb250cmFjdCB3aXRoIGFuIGFkbWluIGFuZCB0aGUgdG9rZW4gdG8gYmUgdmVzdGVkLgAAAAAAAAppbml0aWFsaXplAAAAAAACAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAADXRva2VuX2FkZHJlc3MAAAAAAAATAAAAAQAAA+kAAAPtAAAAAAAAB9AAAAAMVmVzdGluZ0Vycm9y",
            "AAAAAAAAAAAAAAALZ2V0X3Zlc3RpbmcAAAAAAgAAAAAAAAALYmVuZWZpY2lhcnkAAAAAEwAAAAAAAAAFaW5kZXgAAAAAAAAEAAAAAQAAA+gAAAfQAAAAC1Zlc3RpbmdJbmZvAA==",
            "AAAAAQAAAAAAAAAAAAAAC1Zlc3RpbmdJbmZvAAAAAAUAAAAAAAAAB2NsYWltZWQAAAAACwAAAAAAAAAKY2xpZmZfdGltZQAAAAAABgAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAAAAAApzdGFydF90aW1lAAAAAAAGAAAAAAAAAAx0b3RhbF9hbW91bnQAAAAL",
            "AAAAAAAAAFlDcmVhdGVzIGEgbmV3IHZlc3Rpbmcgc2NoZWR1bGUgYW5kIHRyYW5zZmVycyB0aGUgdG90YWxfYW1vdW50IGZyb20gYWRtaW4gdG8gdGhlIGNvbnRyYWN0LgAAAAAAAA5jcmVhdGVfdmVzdGluZwAAAAAABQAAAAAAAAALYmVuZWZpY2lhcnkAAAAAEwAAAAAAAAAMdG90YWxfYW1vdW50AAAACwAAAAAAAAAKc3RhcnRfdGltZQAAAAAABgAAAAAAAAAKY2xpZmZfdGltZQAAAAAABgAAAAAAAAAIZHVyYXRpb24AAAAGAAAAAQAAA+kAAAPtAAAAAAAAB9AAAAAMVmVzdGluZ0Vycm9y",
            "AAAABAAAAAAAAAAAAAAADFZlc3RpbmdFcnJvcgAAAAcAAAAAAAAAEkFscmVhZHlJbml0aWFsaXplZAAAAAAAAQAAAAAAAAAOTm90SW5pdGlhbGl6ZWQAAAAAAAIAAAAAAAAADFVuYXV0aG9yaXplZAAAAAMAAAAAAAAADEludmFsaWRQYXJhbQAAAAQAAAAAAAAACE5vdEZvdW5kAAAABQAAAAAAAAATSW5zdWZmaWNpZW50QmFsYW5jZQAAAAAGAAAAAAAAAAxNYXRoT3ZlcmZsb3cAAAAH",
            "AAAAAAAAAENBZG1pbi1vbmx5OiBXaXRoZHJhdyB0b2tlbnMgZnJvbSB0aGUgY29udHJhY3QgaW4gY2FzZSBvZiBlbWVyZ2VuY3kuAAAAABJlbWVyZ2VuY3lfd2l0aGRyYXcAAAAAAAEAAAAAAAAABmFtb3VudAAAAAAACwAAAAEAAAPpAAAD7QAAAAAAAAfQAAAADFZlc3RpbmdFcnJvcg==",
            "AAAAAAAAADpSZXR1cm5zIHRoZSBhbW91bnQgb2YgdG9rZW5zIGN1cnJlbnRseSBhdmFpbGFibGUgdG8gY2xhaW0uAAAAAAAUZ2V0X2NsYWltYWJsZV9hbW91bnQAAAACAAAAAAAAAAtiZW5lZmljaWFyeQAAAAATAAAAAAAAAAp2ZXN0aW5nX2lkAAAAAAAEAAAAAQAAA+kAAAALAAAH0AAAAAxWZXN0aW5nRXJyb3I="]), options);
        this.options = options;
    }
    fromJSON = {
        claim: (this.txFromJSON),
        initialize: (this.txFromJSON),
        get_vesting: (this.txFromJSON),
        create_vesting: (this.txFromJSON),
        emergency_withdraw: (this.txFromJSON),
        get_claimable_amount: (this.txFromJSON)
    };
}
