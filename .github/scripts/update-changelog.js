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

// Read the existing CHANGELOG.md
let existingChangelog = '';
try {
    existingChangelog = fs.readFileSync(changelogPath, 'utf8');
} catch (error) {
    console.error('Error reading CHANGELOG.md:', error);
    process.exit(1);
}

// Parse the existing CHANGELOG to extract the unreleased section
const unreleasedRegex = /## \[Unreleased\][\s\S]*?(?=## \[v|$)/;
const unreleasedMatch = existingChangelog.match(unreleasedRegex);

if (!unreleasedMatch) {
    console.error('Could not find Unreleased section in CHANGELOG.md');
    process.exit(1);
}

// Get the current date in YYYY-MM-DD format
const currentDate = new Date().toISOString().split('T')[0];

// Create the new version section
const newVersionSection = `## [v${newVersion}] - ${currentDate}\n\n${cleanChangelog}`;

// Update the changelog
// 1. Keep the header (everything before Unreleased)
// 2. Add an empty Unreleased section
// 3. Add the new version section
// 4. Include all previous versions
const headerRegex = /([\s\S]*?)## \[Unreleased\]/;
const headerMatch = existingChangelog.match(headerRegex);
const header = headerMatch ? headerMatch[1] : '';

const previousVersionsRegex = /## \[v.*?\][\s\S]*/;
const previousVersionsMatch = existingChangelog.match(previousVersionsRegex);
const previousVersions = previousVersionsMatch ? previousVersionsMatch[0] : '';

const updatedChangelog = `${header}## [Unreleased] - ReleaseDate

### Added
- 

### Changed
- 

### Fixed
- 

${newVersionSection}

${previousVersions}`;

// Write the updated CHANGELOG.md
try {
    fs.writeFileSync(changelogPath, updatedChangelog);
    console.log(`Successfully updated CHANGELOG.md for version v${newVersion}`);
} catch (error) {
    console.error('Error writing to CHANGELOG.md:', error);
    process.exit(1);
} 