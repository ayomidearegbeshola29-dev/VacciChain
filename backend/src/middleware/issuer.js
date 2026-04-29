const { isAuthorizedIssuer } = require('../stellar/issuerCache');
const logger = require('../logger');

/**
 * Require the authenticated user to have the 'issuer' role AND 
 * verify their wallet is currently authorized on-chain.
 * Must be used after authMiddleware.
 */
async function issuerMiddleware(req, res, next) {
  if (req.user?.role !== 'issuer') {
    return res.status(403).json({ error: 'Issuer role required' });
  }

  const wallet = req.user.wallet || req.user.publicKey;
  if (!wallet) {
    return res.status(401).json({ error: 'Wallet address missing in token' });
  }

  try {
    const isAuthorized = await isAuthorizedIssuer(wallet);
    if (!isAuthorized) {
      logger.warn('Unauthorized issuer attempt', { wallet });
      return res.status(403).json({ error: 'Issuer authorization revoked or not found on-chain' });
    }
    next();
  } catch (error) {
    logger.error('Error verifying issuer allowlist', { wallet, error: error.message });
    res.status(500).json({ error: 'Failed to verify issuer authorization' });
  }
}

module.exports = issuerMiddleware;
