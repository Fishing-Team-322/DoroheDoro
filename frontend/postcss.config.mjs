import path from "node:path";

const projectRoot = path.resolve(
  process.env.npm_package_json
    ? path.dirname(process.env.npm_package_json)
    : process.env.INIT_CWD || process.cwd()
);
const ensureFromPluginPath = path.join(projectRoot, "postcss.ensure-from.cjs");

const config = {
  plugins: {
    [ensureFromPluginPath]: {
      fallbackFrom: path.join(projectRoot, "app", "globals.css"),
    },
    "@tailwindcss/postcss": {
      base: projectRoot,
    },
  },
};

export default config;
