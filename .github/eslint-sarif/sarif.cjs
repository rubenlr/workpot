/**
 * Vendored from @microsoft/eslint-formatter-sarif (sarif.js).
 * Source: https://github.com/microsoft/sarif-js-sdk/tree/main/packages/eslint-formatter-sarif
 * License: MIT — upstream pins eslint@8; we vendor to avoid a duplicate ESLint install with ESLint 9.
 */
/* eslint-disable unicorn/no-null */
"use strict";

const fs = require("fs");
const url = require("url");
const utf8 = require("utf8");
const lodash = require("lodash");
const jschardet = require("jschardet");

function getESLintVersion() {
  try {
    const { ESLint } = require.main.require("eslint");
    return ESLint.version;
  } catch {
    return;
  }
}

function getResultLevel(message) {
  if (message.fatal || message.severity === 2) {
    return "error";
  }
  return "warning";
}

module.exports = function (results, data) {
  const rulesMeta = lodash.get(data, "rulesMeta", null);

  const sarifLog = {
    version: "2.1.0",
    $schema: "http://json.schemastore.org/sarif-2.1.0-rtm.5",
    runs: [
      {
        tool: {
          driver: {
            name: "ESLint",
            informationUri: "https://eslint.org",
            rules: [],
          },
        },
      },
    ],
  };

  const eslintVersion = getESLintVersion();
  if (typeof eslintVersion !== "undefined") {
    sarifLog.runs[0].tool.driver.version = eslintVersion;
  }

  const sarifFiles = {};
  const sarifArtifactIndices = {};
  let nextArtifactIndex = 0;
  const sarifRules = {};
  const sarifRuleIndices = {};
  let nextRuleIndex = 0;
  const sarifResults = [];
  const embedFileContents = process.env.SARIF_ESLINT_EMBED === "true";
  const ignoreSuppressed =
    process.env.SARIF_ESLINT_IGNORE_SUPPRESSED === "true";

  const internalErrorId = "ESL0999";

  const toolConfigurationNotifications = [];
  let executionSuccessful = true;

  for (const result of results) {
    if (typeof sarifFiles[result.filePath] === "undefined") {
      sarifArtifactIndices[result.filePath] = nextArtifactIndex++;

      let contentsUtf8;

      sarifFiles[result.filePath] = {
        location: {
          uri: url.pathToFileURL(result.filePath),
        },
      };

      if (embedFileContents) {
        try {
          const contents = fs.readFileSync(result.filePath);
          const encoding = jschardet.detect(contents);

          if (encoding) {
            contentsUtf8 = utf8.encode(contents.toString(encoding.encoding));

            sarifFiles[result.filePath].contents = {
              text: contentsUtf8,
            };
            sarifFiles[result.filePath].encoding = encoding.encoding;
          }
        } catch (error) {
          console.log(error);
        }
      }

      const containsSuppressedMessages =
        result.suppressedMessages && result.suppressedMessages.length > 0;
      const messages =
        containsSuppressedMessages && !ignoreSuppressed
          ? [...result.messages, ...result.suppressedMessages]
          : result.messages;

      if (messages.length > 0) {
        for (const message of messages) {
          const sarifRepresentation = {
            level: getResultLevel(message),
            message: {
              text: message.message,
            },
            locations: [
              {
                physicalLocation: {
                  artifactLocation: {
                    uri: url.pathToFileURL(result.filePath),
                    index: sarifArtifactIndices[result.filePath],
                  },
                },
              },
            ],
          };

          if (message.ruleId) {
            sarifRepresentation.ruleId = message.ruleId;

            if (
              rulesMeta &&
              typeof sarifRules[message.ruleId] === "undefined"
            ) {
              const meta = rulesMeta[message.ruleId];

              if (meta) {
                sarifRuleIndices[message.ruleId] = nextRuleIndex++;

                if (meta.docs) {
                  sarifRules[message.ruleId] = {
                    id: message.ruleId,
                    helpUri: meta.docs.url,
                    properties: {
                      category: meta.docs.category,
                    },
                  };
                  if (meta.docs.description) {
                    sarifRules[message.ruleId].shortDescription = {
                      text: meta.docs.description,
                    };
                  }
                } else {
                  sarifRules[message.ruleId] = {
                    id: message.ruleId,
                    helpUri: "Please see details in message",
                    properties: {
                      category: "No category provided",
                    },
                  };
                }
              }
            }

            if (sarifRuleIndices[message.ruleId] !== "undefined") {
              sarifRepresentation.ruleIndex = sarifRuleIndices[message.ruleId];
            }

            if (containsSuppressedMessages && !ignoreSuppressed) {
              sarifRepresentation.suppressions = message.suppressions
                ? message.suppressions.map((suppression) => {
                    return {
                      kind:
                        suppression.kind === "directive"
                          ? "inSource"
                          : "external",
                      justification: suppression.justification,
                    };
                  })
                : [];
            }
          } else {
            sarifRepresentation.descriptor = {
              id: internalErrorId,
            };

            if (sarifRepresentation.level === "error") {
              executionSuccessful = false;
            }
          }

          if (message.line > 0 || message.column > 0) {
            sarifRepresentation.locations[0].physicalLocation.region = {};
            if (message.line > 0) {
              sarifRepresentation.locations[0].physicalLocation.region.startLine =
                message.line;
            }
            if (message.column > 0) {
              sarifRepresentation.locations[0].physicalLocation.region.startColumn =
                message.column;
            }
            if (message.endLine > 0) {
              sarifRepresentation.locations[0].physicalLocation.region.endLine =
                message.endLine;
            }
            if (message.endColumn > 0) {
              sarifRepresentation.locations[0].physicalLocation.region.endColumn =
                message.endColumn;
            }
          }

          if (message.source) {
            sarifRepresentation.locations[0].physicalLocation.region =
              sarifRepresentation.locations[0].physicalLocation.region || {};
            sarifRepresentation.locations[0].physicalLocation.region.snippet = {
              text: message.source,
            };
          }

          if (message.ruleId) {
            sarifResults.push(sarifRepresentation);
          } else {
            toolConfigurationNotifications.push(sarifRepresentation);
          }
        }
      }
    }
  }

  if (Object.keys(sarifFiles).length > 0) {
    sarifLog.runs[0].artifacts = [];

    for (const path of Object.keys(sarifFiles)) {
      sarifLog.runs[0].artifacts.push(sarifFiles[path]);
    }
  }

  sarifLog.runs[0].results = sarifResults;

  if (toolConfigurationNotifications.length > 0) {
    sarifLog.runs[0].invocations = [
      {
        toolConfigurationNotifications: toolConfigurationNotifications,
        executionSuccessful: executionSuccessful,
      },
    ];
  }

  if (Object.keys(sarifRules).length > 0) {
    for (const ruleId of Object.keys(sarifRules)) {
      const rule = sarifRules[ruleId];
      sarifLog.runs[0].tool.driver.rules.push(rule);
    }
  }

  return JSON.stringify(sarifLog, null, 2);
};
