import {
  OnHomePageHandler,
  OnInstallHandler,
  OnSignatureHandler,
  OnRpcRequestHandler,
  SeverityLevel,
  MethodNotFoundError,
} from '@metamask/snaps-sdk';
import { Box, Heading, Text, Divider } from '@metamask/snaps-sdk/jsx';

import { Blob, contract_name, HYLE_NODE_URL } from './hyle';
import { AmmAction, deserializeAmmAction, deserializeERC20Action, ERC20Action } from './model';

async function getAccount(): Promise<{ account: string, nonce: number }> {
  // Retrieve stored account
  let state = await snap.request({
    method: 'snap_manageState',
    params: { operation: 'get' },
  });

  // If no stored account, request from MetaMask
  if (!state || !state.account) {
    const accounts = await ethereum.request({ method: 'eth_requestAccounts' }) as string[];

    // If accounts exist, store the first one
    if (accounts.length > 0) {
      state = { account: `${accounts[0]}.${contract_name}`, nonce: 0 };

      // Save account in Snap state
      await snap.request({
        method: 'snap_manageState',
        params: { operation: 'update', newState: state },
      });
    } else {
      state = { account: 'No account found', nonce: 0 };
    }
  }

  return { account: state.account as string, nonce: state.nonce as number };
}

async function bumpNonce() {
  const { account, nonce } = await getAccount();
  await snap.request({
    method: 'snap_manageState',
    params: { operation: 'update', newState: { account, nonce: nonce + 1 } },
  });
}

async function setNonce(nonce: number) {
  const { account } = await getAccount();
  await snap.request({
    method: 'snap_manageState',
    params: { operation: 'update', newState: { account, nonce } },
  });
}

//// Sign message using personal_sign
async function signBlobs(blobs: Array<Blob>) {
  const { account, nonce } = await getAccount();

  const message = `verify ${nonce} ${blobs.map((blob) => blob.contract_name + " [" + blob.data.join(", ") + "]").join(" ")}`;

  const hexMessage = toHexMessage(message); // Convert message to hex

  const ethAddr = await ethereum.request({
    method: 'eth_requestAccounts',
  });

  //try {
  const signature = await ethereum.request<string>({
    method: 'personal_sign',
    params: [hexMessage, ethAddr[0]],
  });

  await bumpNonce(); // Increment nonce after successful signing

  return { account, signature, nonce };
  //} catch (error) {
  //  await snap.request({
  //    method: 'snap_notify',
  //    params: {
  //      type: 'inApp',
  //      message: `Signing failed: ${error.message} with account ${account} and message ${message}`,
  //    },
  //  });
  //  return 'Signing failed';
  //}
}

// Convert message to hex format
function toHexMessage(message: string): string {
  return `0x${Buffer.from(message, 'utf8').toString('hex')}`;
}

function fromHexMessage(hexMessage: string): string {
  if (hexMessage.startsWith('0x')) {
    hexMessage = hexMessage.slice(2);
  }
  return Buffer.from(hexMessage, 'hex').toString('utf8').replace(/\0/g, '');
}

export const onRpcRequest: OnRpcRequestHandler = async ({
  request,
}) => {
  switch (request.method) {
    case "get_account":
      return await getAccount();
    case "sign_blobs":
      return await signBlobs(request.params?.blobs);
    default:
      throw new MethodNotFoundError()
  }
}

// Store account on Snap install
export const onInstall: OnInstallHandler = async () => {
  const { account } = await getAccount();
  const address = account.replace(`.${contract_name}`, "");
  try {
    const response = await fetch(`${HYLE_NODE_URL}/v1/indexer/contract/${contract_name}/nonce/${address}`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });

    const info = await response.json();
    await setNonce(info.nonce);

  } catch (error) {
  }
};

// Use stored account in Home Page
export const onHomePage: OnHomePageHandler = async () => {
  const { account, nonce } = await getAccount();
  let balances = new Map<string, number>();
  try {
    const response = await fetch(`${HYLE_NODE_URL}/v1/indexer/contract/hyllar/state`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });

    const state = await response.json();
    balances.set("hyllar", state.balances[account]);

  } catch (error) {
  }
  try {
    const response = await fetch(`${HYLE_NODE_URL}/v1/indexer/contract/hyllar2/state`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });

    const state = await response.json();
    balances.set("hyllar2", state.balances[account]);

  } catch (error) {
  }

  return {
    content: (
      <Box>
        <Text>Account: {account}</Text>
        <Text>Nonce: {nonce.toString()}</Text>
        <Text>Balances:</Text>
        {Array.from(balances).map(([key, value]) => (
          <Text key={key}>
            {key}: {value?.toString() || "0"}
          </Text>
        ))}
        <Divider />
      </Box>
    ),
  };
};

export const onSignature: OnSignatureHandler = async ({
  signature,
}) => {
  if (typeof signature.data !== 'string') {
    return {
      content: <Text>Invalid signature data</Text>,
      severity: SeverityLevel.Critical,
    };
  }

  try {
    const { blobs, nonce } = parseMessage(fromHexMessage(signature.data));

    const renderInsight = (blob: Blob) => {
      switch (blob.contract_name) {
        case "hyllar":
        case "hyllar2":
          {
            const action = deserializeERC20Action(blob);
            return erc20ActionToInsight(action.parameters);
          }
        case "amm":
          {
            const action = deserializeAmmAction(blob);
            return ammActionToInsight(action.parameters);
          }
        default:
          return (<Text key="unknown">Unknown contract {blob.contract_name} </Text>);
      }
    }

    return {
      content: (
        <Box>
          <Heading>Signature Data:</Heading>
          <Text>Nonce: {nonce.toString()}</Text>
          {blobs.map((blob, index) => (
            <Box key={`${blob.contract_name}-${index}`} >
              <Divider />
              <Heading>Blob #{index.toString()}:</Heading>
              <Text>Contract: {blob.contract_name}</Text>
              <Text>Data: {renderInsight(blob)}</Text>
            </Box>
          ))
          }
        </Box >
      ),
      severity: SeverityLevel.Critical,
    };
  } catch (error) {
    return {
      content: <Text>Invalid signature data: {error.message}</Text>,
      severity: SeverityLevel.Critical,
    };
  }
};

function parseMessage(message: string): { blobs: Blob[], nonce: number } {
  const blobs: Blob[] = [];

  const firstSpace = message.indexOf(' ');
  const nonce = parseInt(message.slice(firstSpace + 1, message.indexOf(' ', firstSpace + 1)), 10);

  // Retirer le nonce et le mot "verify" pour traiter le reste
  const remaining = message.slice(firstSpace + 1 + nonce.toString().length).trim();

  // Séparer les parties en fonction du nom du contrat suivi des crochets
  const regex = /(\w+\d*)\s\[(.*?)\]/g;
  let match;

  while ((match = regex.exec(remaining)) !== null) {
    const contractName = match[1] || ""; // Nom du contrat
    const dataString = match[2] || ""; // Contenu du tableau sous forme de string

    // Convertir la chaîne des données en tableau de nombres
    const data = dataString
      .split(',') // Diviser par la virgule
      .map(num => parseInt(num.trim(), 10)); // Convertir chaque élément en nombre

    // Ajouter le Blob avec le nonce
    blobs.push({
      contract_name: contractName, // Nom du contrat
      data: data,
    });
  }

  return { blobs, nonce };
}

export const erc20ActionToInsight = (action: ERC20Action): string => {
  if ("TotalSupply" in action) {
    return "TotalSupply";
  } else if ("BalanceOf" in action) {
    return `BalanceOf account ${action.BalanceOf.account}`;
  } else if ("Transfer" in action) {
    return `Transfer ${action.Transfer.amount} to ${action.Transfer.recipient}`;
  } else if ("TransferFrom" in action) {
    return `TransferFrom from ${action.TransferFrom.sender} to ${action.TransferFrom.recipient} amount: ${action.TransferFrom.amount}`;
  } else if ("Approve" in action) {
    return `Approve spender: ${action.Approve.spender}, amount: ${action.Approve.amount}`
  } else if ("Allowance" in action) {
    return `Allowance owner: ${action.Allowance.owner}, spender: ${action.Allowance.spender}`;
  }
  return "Unknown ERC20Action"
};

export const ammActionToInsight = (action: AmmAction): string => {
  if ("Swap" in action) {
    return `Swap ${action.Swap.amounts[0]} ${action.Swap.pair[0]} -> ${action.Swap.amounts[1]} ${action.Swap.pair[1]}`;
  } else if ("NewPair" in action) {
    return `NewPair ${action.NewPair.amounts[0]} ${action.NewPair.pair[0]} <-> ${action.NewPair.amounts[1]} ${action.NewPair.pair[1]}`;
  }
  return "Unknown AmmAction"
};
