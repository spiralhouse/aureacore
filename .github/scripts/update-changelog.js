#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Read arguments
const newVersion = process.argv[2];
const cleanChangelog = process.argv[3];

if (!newVersion || !cleanChangelog) {
    console.error('Usage: node update-changelog.js <version> <changelog-content>');
    process.exit(1);
}

const changelogPath = path.join(process.cwd(), 'CHANGELOG.md');

function updateChangelog(filePath, newVersion, changelogContent) {
    try {
        const today = new Date();
        const formattedDate = today.toISOString().split('T')[0];

        console.log(`Updating ${filePath} for version ${newVersion}`);
        console.log(`Using changelog content:\n${changelogContent}`);

        // Read the current changelog
        const changelog = fs.readFileSync(filePath, 'utf8');

        // Process the changelogContent to handle escaped newlines
        const processedChangelog = changelogContent.replace(/\\n/g, '\n');

        // Find the unreleased section
        const unreleasedRegex = /## \[Unreleased\].*?\n(.*?)(?=\n## \[|$)/s;
        const match = changelog.match(unreleasedRegex);

        if (!match) {
            console.error('Could not find Unreleased section in CHANGELOG.md');
            process.exit(1);
        }

        // Clean up the unreleased section, keeping the structure but removing content
        const unreleasedContent = match[1];
        const cleanedUnreleasedContent = unreleasedContent
            .replace(/### (Added|Changed|Fixed|Deprecated|Removed|Security)([\s\S]*?)(?=### |$)/g, (_, title) => {
                return `### ${title}\n- \n\n`;
            })
            .trim() + '\n\n';

        // Create the new version entry
        const versionWithPrefix = newVersion.startsWith('v') ? newVersion : `v${newVersion}`;
        const newVersionEntry = `## [${versionWithPrefix}] - ${formattedDate}\n\n${processedChangelog}\n\n`;

        // Replace the current changelog with the new version and cleaned unreleased section
        const updatedChangelog = changelog.replace(
            unreleasedRegex,
            `## [Unreleased] - ReleaseDate\n${cleanedUnreleasedContent}${newVersionEntry}`
        );

        // Write the updated changelog back to the file
        fs.writeFileSync(filePath, updatedChangelog);
        console.log(`Successfully updated ${filePath} for version ${versionWithPrefix}`);
        return true;
    } catch (error) {
        console.error(`Error updating changelog: ${error.message}`);
        return false;
    }
}

try {
    // Validate the version format
    const versionRegex = /^v?\d+\.\d+\.\d+$/;
    if (!versionRegex.test(newVersion)) {
        console.error(`Invalid version format: ${newVersion}. Expected format: [v]x.y.z`);
        process.exit(1);
    }

    // Call the updateChangelog function with the provided parameters
    const success = updateChangelog(changelogPath, newVersion, cleanChangelog);

    if (!success) {
        process.exit(1);
    }
} catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
} 