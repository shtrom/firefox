import os

# From https://docs.github.com/en/actions/reference/workflows-and-actions/variables
GITHUB_ENV = (
    "CI",
    "GITHUB_ACTION",
    "GITHUB_ACTOR",
    "GITHUB_EVENT_NAME",
    "GITHUB_BASE_REF",
    "GITHUB_HEAD_REF",
    "GITHUB_REF",
    "GITHUB_REF_NAME",
    "GITHUB_REF_TYPE",
    "GITHUB_REF_PROTECTED",
    "GITHUB_REPOSITORY",
    "GITHUB_RUN_ID",
    "GITHUB_WORKFLOW_REF",
)

def main():
    print("Running from Python")

    for key in GITHUB_ENV:
        value = os.environ.get(key)
        print(f"{key} => {value}")

if __name__ == '__main__':
    main()
