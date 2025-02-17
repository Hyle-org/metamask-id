import { borshSerialize, borshDeserialize, BorshSchema, Unit } from 'borsher';
import { Blob, ContractName } from './hyle';

//pub enum IdentityAction {
//    RegisterIdentity { account: String },
//    VerifyIdentity { account: String, nonce: u32 },
//    GetIdentityInfo { account: String },
//}

export type IdentityAction =
  {
    RegisterIdentity: {
      account: string;
    };
  }
  | {
    VerifyIdentity: {
      account: string;
      nonce: number;
    };
  };

export const serialize = (action: IdentityAction): number[] => {
  return Array.from(borshSerialize(schema, action));
};

export const deserializeIdentityAction = (blob: Blob): IdentityAction => {
  return borshDeserialize(schema, Buffer.from(blob.data));
}

const schema = BorshSchema.Enum({
  RegisterIdentity: BorshSchema.Struct({
    account: BorshSchema.String,
  }),
  Circle: BorshSchema.Struct({
    account: BorshSchema.String,
    nonce: BorshSchema.u32,
  }),
});


//pub struct StructuredBlobData<Parameters> {
//    pub caller: Option<BlobIndex>,
//    pub callees: Option<Vec<BlobIndex>>,
//    pub parameters: Parameters,
//}
export type BlobIndex = {
  0: number;
};

export const blobIndexSchema = BorshSchema.Struct({
  0: BorshSchema.u64,
});

export type StructuredBlobData<Parameters> = {
  caller: BlobIndex | null;
  callees: BlobIndex[] | null;
  parameters: Parameters;
};

export const structuredBlobDataSchema = (schema: BorshSchema) =>
  BorshSchema.Struct({
    caller: BorshSchema.Option(blobIndexSchema),
    callees: BorshSchema.Option(BorshSchema.Vec(blobIndexSchema)),
    parameters: schema,
  });


export type ERC20Action =
  | { TotalSupply: {} }
  | { BalanceOf: { account: string } }
  | { Transfer: { recipient: string; amount: number } }
  | { TransferFrom: { sender: string; recipient: string; amount: number } }
  | { Approve: { spender: string; amount: number } }
  | { Allowance: { owner: string; spender: string } };

export const buildTransferBlob = (
  recipient: string,
  token: ContractName,
  amount: number,
): Blob => {
  const action: ERC20Action = {
    Transfer: { recipient, amount },
  };

  const structured: StructuredBlobData<ERC20Action> = {
    caller: null,
    callees: null,
    parameters: action,
  };

  const blob: Blob = {
    contract_name: token,
    data: serializeERC20Action(structured),
  };
  return blob;
};

export const serializeERC20Action = (action: StructuredBlobData<ERC20Action>,): number[] => {
  return Array.from(
    borshSerialize(structuredBlobDataSchema(erc20Schema), action),
  );
};

export const deserializeERC20Action = (blob: Blob): StructuredBlobData<ERC20Action> => {
  return borshDeserialize(structuredBlobDataSchema(erc20Schema), Buffer.from(blob.data));
};

const erc20Schema = BorshSchema.Enum({
  TotalSupply: BorshSchema.Unit,

  BalanceOf: BorshSchema.Struct({
    account: BorshSchema.String,
  }),

  Transfer: BorshSchema.Struct({
    recipient: BorshSchema.String,
    amount: BorshSchema.u128,
  }),

  TransferFrom: BorshSchema.Struct({
    sender: BorshSchema.String,
    recipient: BorshSchema.String,
    amount: BorshSchema.u128,
  }),

  Approve: BorshSchema.Struct({
    spender: BorshSchema.String,
    amount: BorshSchema.u128,
  }),

  Allowance: BorshSchema.Struct({
    owner: BorshSchema.String,
    spender: BorshSchema.String,
  }),
});

// pub enum AmmAction {
//    Swap {
//        pair: TokenPair, // User swaps the first token of the pair for the second token
//        amounts: TokenPairAmount,
//    },
//    NewPair {
//        pair: TokenPair,
//        amounts: TokenPairAmount,
//    },
//}
//
// type TokenPair = (String, String);
// type TokenPairAmount = (u128, u128);

export type TokenPair = [string, string];
export type TokenPairAmount = [number, number];

export type AmmAction =
  | {
    Swap: {
      pair: TokenPair;
      amounts: TokenPairAmount;
    };
  }
  | {
    NewPair: {
      pair: TokenPair;
      amounts: TokenPairAmount;
    };
  };

const ammSchema = BorshSchema.Enum({
  Swap: BorshSchema.Struct({
    pair: BorshSchema.Struct({ 0: BorshSchema.String, 1: BorshSchema.String }),
    amounts: BorshSchema.Struct({ 0: BorshSchema.u128, 1: BorshSchema.u128 }),
  }),
  NewPair: BorshSchema.Struct({
    pair: BorshSchema.Struct({ 0: BorshSchema.String, 1: BorshSchema.String }),
    amounts: BorshSchema.Struct({ 0: BorshSchema.u128, 1: BorshSchema.u128 }),
  }),
});

export const buildSwapBlob = (
  token_a: ContractName,
  token_b: ContractName,
  amount_a: number,
  amount_b: number,
  callees: number[] | null,
): Blob => {
  const action: AmmAction = {
    Swap: { pair: [token_a, token_b], amounts: [amount_a, amount_b] },
  };

  const structured: StructuredBlobData<AmmAction> = {
    caller: null,
    callees,
    parameters: action,
  };

  const blob: Blob = {
    contract_name: "amm",
    data: serializeAmmAction(structured),
  };
  return blob;
};

export const serializeAmmAction = (
  action: StructuredBlobData<AmmAction>,
): number[] => {
  return Array.from(
    borshSerialize(structuredBlobDataSchema(ammSchema), action),
  );
};

export const deserializeAmmAction = (blob: Blob): StructuredBlobData<AmmAction> => {
  return borshDeserialize(structuredBlobDataSchema(ammSchema), Buffer.from(blob.data));
};
