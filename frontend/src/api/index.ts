// frontend/src/api/index.ts
/**
 * Fetches the current application version
 */
export async function getVersion() {
  const response = await fetch("/api/version");
  return await response.text();
}

/**
 * Validates authentication token
 * @param token The auth token to validate
 */
export async function validateAuthToken(token: string) {
  const response = await fetch("/api/auth", {
    method: "GET",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
  });

  return response.ok;
}

/**
 * Sends a cookie to the server.
 * @param cookie The cookie string to send
 * @returns The fetch response object
 *
 * Possible Status Codes:
 * - 200: Success
 * - 400: Invalid cookie
 * - 401: Invalid bearer token
 * - 500: Server error
 */
export async function postCookie(cookie: string) {
  const token = localStorage.getItem("authToken") || "";
  const response = await fetch("/api/submit", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify({ cookie }),
  });

  if (response.status === 400) {
    throw new Error("Invalid cookie format");
  } else if (response.status === 401) {
    throw new Error("Authentication failed. Please set a valid auth token.");
  } else if (response.status === 500) {
    throw new Error("Server error.");
  }

  if (!response.ok) {
    throw new Error(`Error ${response.status}: ${response.statusText}`);
  }

  return response;
}

/**
 * Gets cookie status information from the server.
 * @returns The cookie status data
 *
 * Possible Status Codes:
 * - 200: Success with cookie status data
 * - 401: Invalid bearer token
 * - 500: Server error
 */
export async function getCookieStatus() {
  const token = localStorage.getItem("authToken") || "";
  const response = await fetch("/api/get_cookies", {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    throw new Error(`Error ${response.status}: ${response.statusText}`);
  }

  return await response.json();
}

/**
 * Deletes a cookie from the server.
 * @param cookie The cookie string to delete
 * @returns The fetch response object
 *
 * Possible Status Codes:
 * - 204: Success (No Content)
 * - 401: Invalid bearer token
 * - 500: Server error
 */
export async function deleteCookie(cookie: string) {
  const token = localStorage.getItem("authToken") || "";
  // URL encode the cookie to handle special characters in the URL path
  const encodedCookie = encodeURIComponent(cookie);
  const response = await fetch(`/api/delete_cookie/${encodedCookie}`, {
    method: "DELETE",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });

  return response;
}

/**
 * Fetches the config data from the server
 */
export async function getConfig() {
  const token = localStorage.getItem("authToken") || "";
  const response = await fetch("/api/config", {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch config: ${response.status}`);
  }

  return await response.json();
}

/**
 * Saves config data to the server
 * @param configData The config data to save
 */
export async function saveConfig(configData: any) {
  const token = localStorage.getItem("authToken") || "";
  const response = await fetch("/api/config", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(configData),
  });

  if (!response.ok) {
    throw new Error(`Failed to save config: ${response.status}`);
  }

  return response;
}

// Add this new function to frontend/src/api/index.ts

/**
 * Sends multiple cookies to the server as a batch.
 * @param cookies Array of cookie strings to send
 * @returns An array of results with status for each cookie
 */
export async function postMultipleCookies(cookies: string[]) {
  const token = localStorage.getItem("authToken") || "";
  const results = [];

  for (const cookie of cookies) {
    try {
      const response = await fetch("/api/submit", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ cookie }),
      });

      if (response.status === 400) {
        results.push({
          cookie,
          success: false,
          message: "Invalid cookie format",
        });
      } else if (response.status === 401) {
        results.push({
          cookie,
          success: false,
          message: "Authentication failed. Please set a valid auth token.",
        });
      } else if (response.status === 500) {
        results.push({
          cookie,
          success: false,
          message: "Server error.",
        });
      } else if (!response.ok) {
        results.push({
          cookie,
          success: false,
          message: `Error ${response.status}: ${response.statusText}`,
        });
      } else {
        results.push({
          cookie,
          success: true,
          message: "Cookie submitted successfully",
        });
      }
    } catch (error) {
      results.push({
        cookie,
        success: false,
        message: error instanceof Error ? error.message : "Unknown error",
      });
    }
  }

  return results;
}
