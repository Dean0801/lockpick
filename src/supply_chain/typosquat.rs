/// Damerau-Levenshtein distance (supports transposition)
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());

    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, val) in d[0].iter_mut().enumerate().take(m + 1) {
        *val = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}

/// Check a package name against popular packages list.
/// Returns Some((similar_pkg, distance)) if suspicious.
pub fn check_typosquat(name: &str) -> Option<(String, usize)> {
    if name.len() <= 2 {
        return None; // too short, high false-positive risk
    }
    let threshold = if name.len() <= 3 { 1 } else { 2 };
    let mut best: Option<(String, usize)> = None;

    for &popular in POPULAR_PACKAGES {
        if name == popular {
            return None;
        } // exact match = not typosquat
        let dist = edit_distance(name, popular);
        if dist > 0 && dist <= threshold && best.as_ref().is_none_or(|(_, d)| dist < *d) {
            best = Some((popular.to_string(), dist));
        }
    }
    best
}

pub const POPULAR_PACKAGES: &[&str] = &[
    "react",
    "react-dom",
    "next",
    "vue",
    "angular",
    "express",
    "koa",
    "fastify",
    "hapi",
    "nest",
    "lodash",
    "underscore",
    "ramda",
    "moment",
    "dayjs",
    "axios",
    "node-fetch",
    "got",
    "request",
    "superagent",
    "webpack",
    "rollup",
    "vite",
    "esbuild",
    "parcel",
    "babel",
    "typescript",
    "eslint",
    "prettier",
    "jest",
    "mocha",
    "chai",
    "vitest",
    "cypress",
    "playwright",
    "redux",
    "mobx",
    "zustand",
    "recoil",
    "jotai",
    "tailwindcss",
    "postcss",
    "sass",
    "less",
    "styled-components",
    "mongoose",
    "sequelize",
    "prisma",
    "typeorm",
    "knex",
    "socket.io",
    "ws",
    "graphql",
    "apollo",
    "urql",
    "chalk",
    "commander",
    "yargs",
    "inquirer",
    "ora",
    "dotenv",
    "cors",
    "helmet",
    "morgan",
    "compression",
    "jsonwebtoken",
    "bcrypt",
    "passport",
    "cookie-parser",
    "body-parser",
    "uuid",
    "nanoid",
    "date-fns",
    "luxon",
    "cheerio",
    "puppeteer",
    "sharp",
    "jimp",
    "multer",
    "formidable",
    "nodemailer",
    "bull",
    "ioredis",
    "redis",
    "pg",
    "mysql2",
    "sqlite3",
    "mongodb",
    "cassandra-driver",
    "couchbase",
    "winston",
    "pino",
    "bunyan",
    "debug",
    "log4js",
    "rxjs",
    "immer",
    "zod",
    "yup",
    "joi",
    "semver",
    "glob",
    "minimatch",
    "micromatch",
    "picomatch",
    "fs-extra",
    "rimraf",
    "mkdirp",
    "chokidar",
    "globby",
    "execa",
    "cross-env",
    "concurrently",
    "npm-run-all",
    "husky",
    "lint-staged",
    "commitlint",
    "semantic-release",
    "lerna",
    "turbo",
    "storybook",
    "chromatic",
    "msw",
    "nock",
    "sinon",
    "enzyme",
    "testing-library",
    "supertest",
    "ava",
    "tap",
    "electron",
    "tauri",
    "capacitor",
    "ionic",
    "expo",
    "three",
    "d3",
    "chart.js",
    "echarts",
    "highcharts",
    "swiper",
    "slick-carousel",
    "animate.css",
    "framer-motion",
    "gsap",
    "i18next",
    "intl",
    "numeral",
    "accounting",
    "currency.js",
    "classnames",
    "clsx",
    "prop-types",
    "immutable",
    "memoize-one",
    "path-to-regexp",
    "qs",
    "query-string",
    "url-parse",
    "punycode",
    "mime",
    "content-type",
    "accepts",
    "negotiator",
    "vary",
    "http-errors",
    "statuses",
    "raw-body",
    "on-finished",
    "destroy",
    "depd",
    "fresh",
    "etag",
    "proxy-addr",
    "forwarded",
    "send",
    "serve-static",
    "finalhandler",
    "parseurl",
    "encodeurl",
    "escape-html",
    "merge-descriptors",
    "methods",
    "range-parser",
    "type-is",
    "npm",
    "yarn",
    "pnpm",
    "bun",
    "deno",
    "webpack-cli",
    "webpack-dev-server",
    "html-webpack-plugin",
    "mini-css-extract-plugin",
    "css-loader",
    "style-loader",
    "file-loader",
    "url-loader",
    "babel-loader",
    "ts-loader",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance_identical() {
        assert_eq!(edit_distance("lodash", "lodash"), 0);
    }

    #[test]
    fn test_edit_distance_one_char() {
        assert_eq!(edit_distance("lodash", "lodas"), 1);
    }

    #[test]
    fn test_edit_distance_transpose() {
        assert_eq!(edit_distance("lodash", "ldoash"), 1);
    }

    #[test]
    fn test_check_typosquat_hit() {
        let result = check_typosquat("lod-ash");
        assert!(result.is_some());
        let (pkg, _dist) = result.unwrap();
        assert_eq!(pkg, "lodash");
    }

    #[test]
    fn test_check_typosquat_exact_no_alert() {
        assert!(check_typosquat("lodash").is_none());
    }

    #[test]
    fn test_check_typosquat_too_far() {
        assert!(check_typosquat("completely-different-name").is_none());
    }
}
