module.exports = function ensurePostcssFrom(options = {}) {
  return {
    postcssPlugin: "ensure-postcss-from",
    Once(root, { result }) {
      // Turbopack sometimes evaluates CSS with an empty `from`, which makes
      // Tailwind resolve `@import "tailwindcss"` from the wrong working dir.
      result.opts.from ??= root.source?.input?.file ?? options.fallbackFrom;
    },
  };
};
