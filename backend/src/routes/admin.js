const express = require('express');
const crypto = require('crypto');
const authMiddleware = require('../middleware/auth');
const { queryAuditLog } = require('../middleware/auditLog');
const { insertApiKey, listApiKeys, revokeApiKey } = require('../indexer/db');
const { approveProposal, getProposal } = require('../middleware/multiSig');

const router = express.Router();

function adminOnly(req, res, next) {
  if (req.user?.role !== 'issuer') {
    return res.status(403).json({ error: 'Admin access required' });
  }
  next();
}

/**
 * GET /admin/audit
 */
router.get('/audit', authMiddleware, adminOnly, (req, res) => {
  const { actor, from, to } = req.query;
  const limit  = Math.min(parseInt(req.query.limit  || '100', 10), 1000);
  const offset = Math.max(parseInt(req.query.offset || '0',   10), 0);

  if (from && isNaN(new Date(from).getTime())) {
    return res.status(400).json({ error: 'Invalid "from" date' });
  }
  if (to && isNaN(new Date(to).getTime())) {
    return res.status(400).json({ error: 'Invalid "to" date' });
  }

  const entries = queryAuditLog({ actor, from, to });
  const page    = entries.slice(offset, offset + limit);
  res.json({ total: entries.length, offset, limit, entries: page });
});

// ── API key management ────────────────────────────────────────────────────────

/**
 * POST /admin/api-keys
 * Body: { label: string }
 * Returns the raw key once — it is never stored in plaintext.
 */
router.post('/api-keys', authMiddleware, adminOnly, (req, res) => {
  const { label } = req.body;
  if (!label || typeof label !== 'string' || !label.trim()) {
    return res.status(400).json({ error: 'label is required' });
  }

  const rawKey  = crypto.randomBytes(32).toString('hex');
  const keyHash = crypto.createHash('sha256').update(rawKey).digest('hex');
  const id      = crypto.randomUUID();

  insertApiKey({ id, key_hash: keyHash, label: label.trim(), created_at: new Date().toISOString() });

  res.status(201).json({ id, label: label.trim(), key: rawKey });
});

/**
 * GET /admin/api-keys
 * Returns all keys (without the raw key value).
 */
router.get('/api-keys', authMiddleware, adminOnly, (_req, res) => {
  res.json(listApiKeys());
});

/**
 * DELETE /admin/api-keys/:id
 * Revokes a key by id.
 */
router.delete('/api-keys/:id', authMiddleware, adminOnly, (req, res) => {
  revokeApiKey(req.params.id);
  res.json({ revoked: true });
});

// ── Multi-sig proposal management ─────────────────────────────────────────────

/**
 * POST /admin/multisig/approve
 * Body: { proposal_id: string }
 *
 * A registered key holder approves a pending multi-sig proposal.
 * Once MULTISIG_THRESHOLD approvals are collected the proposal is marked
 * "approved" and the initiator can re-submit the original request with
 * the proposal_id to execute it.
 */
router.post('/multisig/approve', authMiddleware, adminOnly, (req, res) => {
  const { proposal_id } = req.body;
  if (!proposal_id) {
    return res.status(400).json({ error: 'proposal_id is required' });
  }

  try {
    const proposal = approveProposal(proposal_id, req.user.wallet);
    res.json({
      proposal_id: proposal.id,
      operation: proposal.operation,
      approvals: proposal.approvals.size,
      status: proposal.status,
      expires_at: new Date(proposal.expiresAt).toISOString(),
    });
  } catch (err) {
    const status = err.message.includes('not found') ? 404
      : err.message.includes('expired') ? 410
      : err.message.includes('not a registered') ? 403
      : 400;
    res.status(status).json({ error: err.message });
  }
});

/**
 * GET /admin/multisig/proposals/:id
 * Returns the current state of a proposal (approval count, status, expiry).
 */
router.get('/multisig/proposals/:id', authMiddleware, adminOnly, (req, res) => {
  const proposal = getProposal(req.params.id);
  if (!proposal) {
    return res.status(404).json({ error: 'Proposal not found or expired' });
  }
  res.json({
    proposal_id: proposal.id,
    operation: proposal.operation,
    initiator: proposal.initiator,
    approvals: proposal.approvals.size,
    status: proposal.status,
    expires_at: new Date(proposal.expiresAt).toISOString(),
  });
});

module.exports = router;
