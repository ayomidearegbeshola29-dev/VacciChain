const StellarSdk = require('@stellar/stellar-sdk');
const config = require('../config');
const logger = require('../logger');

const {
  SOROBAN_RPC_URL,
  STELLAR_NETWORK_PASSPHRASE: NETWORK_PASSPHRASE,
  VACCINATIONS_CONTRACT_ID: CONTRACT_ID,
  SOROBAN_RPC_MAX_RETRIES,
} = config;

function getRpcServer() {
  return new StellarSdk.SorobanRpc.Server(SOROBAN_RPC_URL);
}

/**
 * Helper to retry RPC calls with exponential backoff.
 * @param {Function} fn - The RPC call to execute
 * @param {string} context - Context for logging
 */
async function withRetry(fn, context = '') {
  let lastError;
  for (let attempt = 0; attempt <= SOROBAN_RPC_MAX_RETRIES; attempt++) {
    try {
      if (attempt > 0) {
        const backoff = Math.pow(2, attempt - 1) * 100; // 100ms, 200ms, 400ms
        logger.info(`Retrying Soroban RPC call`, {
          context,
          attempt,
          maxRetries: SOROBAN_RPC_MAX_RETRIES,
          backoffMs: backoff,
        });
        await new Promise((resolve) => setTimeout(resolve, backoff));
      }
      return await fn();
    } catch (error) {
      lastError = error;

      // Don't retry 4xx errors as they are likely client-side/contract logic errors
      const status = error.response?.status;
      if (status && status >= 400 && status < 500) {
        logger.warn(`Non-retryable Soroban RPC error`, {
          context,
          status,
          message: error.message,
        });
        throw error;
      }

      if (attempt === SOROBAN_RPC_MAX_RETRIES) {
        logger.error(`Soroban RPC call failed after maximum retries`, {
          context,
          attempt,
          message: error.message,
        });
      } else {
        logger.debug(`Soroban RPC call transient failure`, {
          context,
          attempt,
          message: error.message,
        });
      }
    }
  }
  throw lastError;
}

/**
 * Invoke a Soroban contract function.
 * @param {string} secretKey - Caller's secret key
 * @param {string} method - Contract method name
 * @param {StellarSdk.xdr.ScVal[]} args - Method arguments
 */
async function invokeContract(secretKey, method, args) {
  const rpc = getRpcServer();
  const keypair = StellarSdk.Keypair.fromSecret(secretKey);
  const account = await withRetry(() => rpc.getAccount(keypair.publicKey()), 'getAccount');

  const contract = new StellarSdk.Contract(CONTRACT_ID);

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const prepared = await withRetry(() => rpc.prepareTransaction(tx), 'prepareTransaction');
  prepared.sign(keypair);

  const response = await withRetry(() => rpc.sendTransaction(prepared), 'sendTransaction');
  if (response.status === 'ERROR') {
    throw new Error(`Contract invocation failed: ${JSON.stringify(response.errorResult)}`);
  }

  // Poll for result
  let result;
  for (let i = 0; i < 10; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    result = await withRetry(() => rpc.getTransaction(response.hash), 'getTransaction');
    if (result.status !== 'NOT_FOUND') break;
  }

  if (result.status !== 'SUCCESS') {
    throw new Error(`Transaction failed: ${result.status}`);
  }

  return result.returnValue;
}

/**
 * Read-only contract call (no signing needed).
 */
async function simulateContract(method, args) {
  const rpc = getRpcServer();
  const contract = new StellarSdk.Contract(CONTRACT_ID);

  // Use a dummy account for simulation
  const dummyKeypair = StellarSdk.Keypair.random();
  const account = new StellarSdk.Account(dummyKeypair.publicKey(), '0');

  const tx = new StellarSdk.TransactionBuilder(account, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const sim = await withRetry(() => rpc.simulateTransaction(tx), 'simulateTransaction');
  if (StellarSdk.SorobanRpc.Api.isSimulationError(sim)) {
    throw new Error(`Simulation failed: ${sim.error}`);
  }

  return sim.result?.retval;
}

module.exports = { invokeContract, simulateContract };
