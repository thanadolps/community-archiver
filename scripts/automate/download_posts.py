import argparse
import json
from pathlib import Path
from time import sleep

import browser_cookie3
from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.remote.webdriver import WebDriver


def setup_driver():
    """Setup Chrome driver with dark mode enabled."""
    options = Options()
    # Enable dark mode
    options.add_argument("--force-dark-mode")
    options.add_experimental_option(
        "prefs",
        {
            "profile.default_content_setting_values.automatic_downloads": 1,
        },
    )

    driver = webdriver.Chrome(options=options)
    driver.set_script_timeout(3600)  # 60 minutes
    driver.set_page_load_timeout(3600)

    # Set a longer timeout for the HTTP connection (urllib3 level)
    # This fixes the "Read timed out. (read timeout=120)" error
    if hasattr(driver, "command_executor") and hasattr(
        driver.command_executor, "_client_config"
    ):
        client_config = driver.command_executor._client_config  # type: ignore
        if hasattr(client_config, "timeout"):
            client_config.timeout = 3600  # type: ignore

    return driver


def load_cookies(driver: WebDriver, browser="chromium"):
    """Load cookies from specified browser.

    Args:
        driver: Selenium WebDriver instance
        browser: Browser to load cookies from (chromium, chrome, firefox, edge, etc.)
    """
    print(f"Loading cookies from {browser}...")

    # Get the appropriate browser cookie function
    browser_func = getattr(browser_cookie3, browser.lower(), None)
    if browser_func is None:
        raise ValueError(
            f"Unknown browser: {browser}. Available: chromium, chrome, firefox, edge, safari, opera, etc."
        )

    cj = browser_func()

    # Go to YouTube first
    driver.get("https://www.youtube.com")
    sleep(2)

    cookie_count = 0
    for cookie in cj:
        if cookie.domain in (
            ".google.com",
            "google.com",
            ".youtube.com",
            "youtube.com",
        ):
            cookie_dict = {
                "name": cookie.name,
                "value": cookie.value,
                "path": cookie.path,
            }

            if cookie.domain.startswith("."):
                cookie_dict["domain"] = cookie.domain

            if hasattr(cookie, "secure") and cookie.secure:
                cookie_dict["secure"] = True

            if cookie.expires:
                cookie_dict["expiry"] = int(cookie.expires)

            try:
                driver.add_cookie(cookie_dict)
                cookie_count += 1
            except Exception:
                pass

    driver.refresh()
    sleep(3)
    print(f"✓ Loaded {cookie_count} cookies")


def get_missing_post_ids(
    limit: int | None = None, specific_ids: list[str] | None = None
):
    """Get list of missing post IDs that need to be downloaded.

    Args:
        limit: Maximum number of posts to return (for testing)
        specific_ids: List of specific post IDs to process
    """
    print("\nChecking for missing posts...")

    # Load source list from post_ids.json
    data_path = Path(__file__).parent.parent.parent / "data" / "post_ids.json"
    with open(data_path) as fs:
        source_list = json.load(fs)
    source = set(source_list)

    # Get already downloaded posts from archive
    archive_path = Path(__file__).parent.parent.parent / "archive"
    archive_path.mkdir(exist_ok=True)

    dest = set()
    for fs in archive_path.iterdir():
        if fs.is_dir():
            continue
        post_id = fs.stem
        dest.add(post_id)

    # If specific IDs provided, only process those
    if specific_ids:
        print(f"Processing {len(specific_ids)} specific post(s)")
        # Filter to only include IDs that exist in source
        valid_ids = [pid for pid in specific_ids if pid in source]
        if len(valid_ids) < len(specific_ids):
            invalid = set(specific_ids) - set(valid_ids)
            print(
                f"⚠️  Warning: {len(invalid)} ID(s) not found in post_ids.json: {invalid}"
            )
        return valid_ids

    # Calculate missing posts
    missing = source - dest

    # Sort by original order in source_list
    missing_sorted = sorted(missing, key=lambda x: source_list.index(x))

    print(f"Total posts: {len(source_list)}")
    print(f"Already downloaded: {len(dest)}")
    print(f"Missing: {len(missing_sorted)}")

    # Apply limit if specified
    if limit:
        print(f"\n⚠️  Limiting to first {limit} post(s)")
        return missing_sorted[:limit]

    return missing_sorted


def extract_and_save_post(
    driver: WebDriver,
    post_id: str,
    index: int,
    total: int,
    js_code: str,
    archive_path: Path,
):
    """Navigate to post, extract content, and save to archive."""
    url = f"https://www.youtube.com/post/{post_id}"
    print(f"\n[{index}/{total}] Processing: {post_id}")

    # Navigate to the post
    driver.get(url)
    sleep(3)

    # Execute extract_post() and wait for completion
    try:
        result = driver.execute_async_script(f"""
            {js_code}

            const callback = arguments[arguments.length - 1];
            extract_post()
                .then(result => callback(result))
                .catch(err => callback({{error: err.message}}));
        """)
    except Exception as e:
        print(f"  ✗ Failed: Selenium timeout or error - {str(e)[:100]}")
        return False

    # Only save if extraction was successful
    if result is False:
        print("  ✗ Failed: Extraction returned false (likely image loading timeout)")
        return False
    elif result and isinstance(result, dict) and not result.get("error"):
        extracted_id = result.get("id")
        html_content = result.get("html")

        if extracted_id and html_content:
            # Save to archive folder
            output_file = archive_path / f"{extracted_id}.html"
            with open(output_file, "w", encoding="utf-8") as f:
                f.write(html_content)

            file_size = len(html_content) / 1024 / 1024  # MB
            print(f"  ✓ Saved: {extracted_id}.html ({file_size:.2f} MB)")
            return True
        else:
            print("  ✗ Failed: Missing id or html in result")
            return False
    elif isinstance(result, dict) and result.get("error"):
        print(f"  ✗ Failed: {result.get('error')}")
        return False
    else:
        print(f"  ✗ Failed: Unexpected result type: {type(result)}")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Download YouTube community posts to archive folder"
    )
    parser.add_argument(
        "--limit",
        type=int,
        help="Limit number of posts to download (for testing)",
    )
    parser.add_argument(
        "--posts",
        nargs="+",
        help="Specific post ID(s) to download (space-separated)",
    )
    parser.add_argument(
        "--browser",
        default="chromium",
        help="Browser to load cookies from (default: chromium). Options: chromium, chrome, firefox, edge, safari, opera",
    )
    args = parser.parse_args()

    driver = setup_driver()

    try:
        # Load cookies for authentication
        load_cookies(driver, browser=args.browser)

        # Load the JavaScript file
        js_path = Path(__file__).parent.parent / "browser" / "download_posts.js"
        with open(js_path, "r") as f:
            js_code = f.read()

        # Archive folder path
        archive_path = Path(__file__).parent.parent.parent / "archive"

        # Get missing post IDs
        missing_ids = get_missing_post_ids(limit=args.limit, specific_ids=args.posts)

        if len(missing_ids) == 0:
            print("\n✓ All posts are already downloaded!")
            return

        print(f"\n{'=' * 60}")
        print(f"Starting download of {len(missing_ids)} posts...")
        print(f"{'=' * 60}")

        # Process each missing post
        success_count = 0
        fail_count = 0

        for i, post_id in enumerate(missing_ids, 1):
            success = extract_and_save_post(
                driver, post_id, i, len(missing_ids), js_code, archive_path
            )
            if success:
                success_count += 1
            else:
                fail_count += 1
            sleep(2)

        print(f"\n{'=' * 60}")
        print("Download complete!")
        print(f"  Success: {success_count}")
        print(f"  Failed: {fail_count}")
        print(f"  Total: {len(missing_ids)}")
        print(f"{'=' * 60}")

        sleep(5)

    finally:
        driver.quit()


if __name__ == "__main__":
    main()
