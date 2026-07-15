import express from "express";
import path from "path";
import { createServer as createViteServer } from "vite";
import compression from "compression";
import rateLimit from "express-rate-limit";

async function startServer() {
  const app = express();
  const PORT = 3000;

  // Commercial-grade middlewares
  app.use(compression());
  app.use(express.json({ limit: "50mb" }));
  app.use(express.urlencoded({ extended: true, limit: "50mb" }));
  
  // Rate limiting to prevent abuse
  const limiter = rateLimit({
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 1000, // Limit each IP to 1000 requests per windowMs
    standardHeaders: true, // Return rate limit info in the `RateLimit-*` headers
    legacyHeaders: false, // Disable the `X-RateLimit-*` headers
    message: { success: false, error: "Too many requests from this IP, please try again after 15 minutes", code: 429 }
  });
  app.use("/api", limiter);

  // Custom CORS & Security Headers
  app.use((req, res, next) => {
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("X-XSS-Protection", "1; mode=block");
    res.setHeader("Strict-Transport-Security", "max-age=31536000; includeSubDomains");
    next();
  });

  // Example backend API router mapping
  const apiRouter = express.Router();
  
  apiRouter.get("/health", (req, res) => {
    res.json({ status: "ok", timestamp: new Date().toISOString(), version: "1.2.0" });
  });

  apiRouter.get("/agents", (req, res) => {
    res.json({
      success: true,
      data: [
        { id: "codex-engine", name: "Codex AI Engine", status: "installed" },
        { id: "gemini-core", name: "Gemini Core Services", status: "setup_required" }
      ]
    });
  });

  apiRouter.post("/telemetry", (req, res) => {
    // Process analytics/telemetry payload here
    res.status(202).json({ success: true, message: "Telemetry accepted" });
  });

  app.use('/api', apiRouter);

  // Error handler middleware
  app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
    console.error('[Server Error]', err.stack);
    res.status(500).json({ success: false, error: 'Internal Server Protocol Exception', code: 500 });
  });

  // Vite middleware for development
  if (process.env.NODE_ENV !== "production") {
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: "spa",
    });
    app.use(vite.middlewares);
  } else {
    const distPath = path.join(process.cwd(), 'dist');
    app.use(express.static(distPath));
    app.get('*all', (req, res) => {
      res.sendFile(path.join(distPath, 'index.html'));
    });
  }

  app.listen(PORT, "0.0.0.0", () => {
    console.log(`Server running on http://0.0.0.0:${PORT}`);
  });
}

startServer();
