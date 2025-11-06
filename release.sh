#!/bin/bash

RELEASE_BRANCH="rust"

# 1. Check for required Git branch
if [ "$(git rev-parse --abbrev-ref HEAD)" != "$RELEASE_BRANCH" ]; then
    echo "Error: You must be on the '$RELEASE_BRANCH' branch to create a release tag."
    exit 1
fi

# 2. Pull the latest code and tags
echo "Pulling latest code and tags from origin/$RELEASE_BRANCH..."
git pull origin "$RELEASE_BRANCH" --tags

# 3. Find the last tag
# The 2>/dev/null suppresses the "No tags found" error if it's the first tag
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null)

if [ -z "$LAST_TAG" ]; then
    echo "No previous tags found. Suggesting v1.0.0"
else
    echo "Last tag published: $LAST_TAG"
fi

# 4. Prompt for the new tag
while true; do
    read -rp "Enter the new tag to push (e.g., v1.0.1, or 'exit' to cancel): " NEW_TAG

    if [ "$NEW_TAG" == "exit" ]; then
        echo "Canceled."
        exit 0
    elif [ -z "$NEW_TAG" ]; then
        echo "Tag cannot be empty. Please try again."
    elif [[ "$NEW_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+.* ]]; then
        # Basic check to ensure it looks like a SemVer tag (vX.Y.Z)
        break
    else
        echo "Invalid format. Tag must start with 'v' followed by numbers (e.g., v1.2.3)."
    fi
done

# 5. Create and push the tag
echo "Creating and pushing tag $NEW_TAG..."
git tag -a "$NEW_TAG" -m "Release $NEW_TAG"

# Push the tag to GitHub, triggering the GitHub Actions workflow
if git push origin "$NEW_TAG"; then
    echo "Success. Tag $NEW_TAG pushed to GitHub."
    echo "The multi-platform compilation workflow has been triggered."
else
    echo "Failed to push tag. Deleting local tag to retry."
    git tag -d "$NEW_TAG"
    exit 1
fi
