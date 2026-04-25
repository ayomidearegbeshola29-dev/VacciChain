require('dotenv').config();
require('./config');
const express = require('express');
const cors = require('cors');
const logger = require('./logger');

const authRoutes = require('./routes/auth');
const vaccinationRoutes = require('./routes/vaccination');
const verifyRoutes = require('./routes/verify');

const app = express();

app.use(cors());
app.use(express.json());

// Request logging middleware
app.use((req, _res, next) => {
  logger.info('request', { method: req.method, path: req.path });
  next();
});

app.use('/auth', authRoutes);
app.use('/vaccination', vaccinationRoutes);
app.use('/verify', verifyRoutes);

app.get('/health', (_req, res) => res.json({ status: 'ok' }));

// Error logging middleware
app.use((err, _req, res, _next) => {
  logger.error('unhandled error', { error: err.message, stack: err.stack });
  res.status(500).json({ error: 'Internal server error' });
});

const PORT = process.env.PORT || 4000;
if (require.main === module) {
  app.listen(PORT, () => logger.info('Backend running', { port: PORT }));
}

module.exports = app;
