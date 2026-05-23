"use strict";

var authWindow = null; // Yech!
var gDriveAuthPending = true;
var turnstileWidgetId = undefined;
var postInProgress = false;
function HandleGAuthComplete(result) {
  if (result === "success") {
    if (document.getElementById("pasteform").reportValidity()) {
      gDriveAuthPending = false;
      document.getElementById("pastebtn").click();
    }
  } else {
    gDriveAuthPending = true;
    document.getElementById("pastebtn").classList.remove("btn-primary");
    document.getElementById("pastebtn").classList.add("btn-danger");

    document.getElementById("pastebtn-loading").classList.add("d-none");
    document.getElementById("pastebin-error-text").textContent = result;
    document.getElementById("pastebtn-error").classList.remove("d-none");
  }
}

async function fancyFormSubmit(token) {
  let form = document.getElementById("pasteform");
  let data = new FormData(form);
  data.set("token", token);

  if (document.querySelector("input[name='encrypted']").value === "true") {
    try {
      // Generate a random key
      const key = await generateEncryptionKey();

      // Get the content from the textarea
      const contentElement = document.getElementById("content");
      const originalContent = contentElement.value;

      // Encrypt the content
      const encryptedContent = await encryptContent(originalContent, key);

      // Export the key to base64
      const keyBase64 = await exportKey(key);

      // Replace the content with encrypted version
      data.set("content", encryptedContent);

      // Store the key for URL fragment (will be added after redirect)
      window.encryptionKey = keyBase64;
    } catch (e) {
      window.encryptionKey = null; // Clean up
      console.error('Encryption failed:', e);
      alert('Oops, encryption failed! Please try again or disable encryption.');
      document.getElementById("pastebtn-loading").classList.add("d-none");
      document.getElementById("pastebtn-ready").classList.remove("d-none");
      document.getElementById("pasteform-fields").removeAttribute("disabled");
      document.getElementById("content").focus();
      return;
    }
  };

  // Encode the form data using URLSearchParams
  const encodedData = new URLSearchParams(data);

  document.getElementById("pasteform-fields").setAttribute("disabled", true);
  fetch(form.getAttribute("action"), {
    method: form.getAttribute("method"),
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
      "X-Requested-With": "XMLHttpRequest",
    },
    body: encodedData.toString(),
  })
    .then((response) => {
      if (response.ok) {
        return response.text();
      } else {
        let alertText = `Oops, we couldn't post your paste :( The following was encountered:\n\n${response.status}: ${response.statusText}`;
        if (response.status == 403) {
          alertText += "\n\nMaybe clear your cookies and refresh the page?";
        }
        alert(alertText);
        throw "-flails-";
      }
    })
    .then((result) => {
      // If we encrypted the content, append the key as a URL fragment
      if (window.encryptionKey) {
        location.replace(result + "#" + window.encryptionKey);
        window.encryptionKey = null; // Clean up
      } else {
        location.replace(result);
      }
    })
    .catch((error) => {
      if (error != "-flails-") {
        alert(
          "Oops, we couldn't post your paste :( Maybe the network pipes aren't up?",
        );
      }
      document.getElementById("pastebtn-loading").classList.add("d-none");
      document.getElementById("pastebtn-ready").classList.remove("d-none");

      document.getElementById("pasteform-fields").removeAttribute("disabled");
      document.getElementById("content").focus();

      if (turnstileWidgetId != undefined) {
        turnstile.remove(turnstileWidgetId);
        turnstileWidgetId = undefined;
        postInProgress = false;
      }
    });
}

(function () {
  window.addEventListener("DOMContentLoaded", () => {
    // Character counter
    document.getElementById("content").addEventListener("input", function () {
      document.getElementById("noc").textContent = this.value.length;
      try {
        api_popover.hide();
      } catch (e) {};
    });

    // Encrypted paste thingie!
    document.getElementById("paste-w-encryption").addEventListener("click", (e) => {
      e.preventDefault();

      let encrypt = confirm("Encrypted pastes are removed after 14 days of inactivity!");
      if (encrypt) {
        document.querySelector("input[name='encrypted']").value = "true";
        document.getElementById("pastebtn").click();
      } else {
        document.querySelector("input[name='encrypted']").value = "false";
      }
    });

    // Fancy form submit
    document.getElementById("pasteform").addEventListener("submit", (e) => {
      e.preventDefault();

      document.getElementById("pastebtn-ready").classList.add("d-none");
      document.getElementById("pastebtn-loading").classList.remove("d-none");

      if (
        document.querySelector("input[name=destination]:checked").value ==
        "gdrive"
      ) {
        if (authWindow != null && !authWindow.closed) {
          authWindow.close();
        }

        if (gDriveAuthPending) {
          let gauthUrl = "/pastebin/auth/gdrive/start";
          authWindow = window.open(gauthUrl, "gauthFrame");
          if (authWindow == null) {
            alert(
              "Oops, our little popup couldn't popup! Mebbe you need to allow the popup?",
            );
            document.getElementById("pastebtn-loading").classList.add("d-none");
            document
              .getElementById("pastebtn-ready")
              .classList.remove("d-none");
          } else {
            authWindow.focus();
          }
          return;
        }
      }

      if (turnstileWidgetId != undefined || postInProgress) {
        return; // bail out if it's already in progress
      }

      postInProgress = true;
      let rkey = document.getElementById("recaptcha-key").value;
      if (rkey.length > 0) {
        turnstileWidgetId = turnstile.render("#cf-turnstile", {
          sitekey: rkey,
          action: "paste",
          theme: "dark",
          callback: fancyFormSubmit,
        });
      } else {
        fancyFormSubmit("");
      };
    });

    // Keyboard accelerators
    document.addEventListener("keydown", (e) => {
      if (e.ctrlKey && e.key == "Enter") {
        document.getElementById("plain").checked = true;
        if (document.getElementById("pasteform").reportValidity()) {
          document.getElementById("pastebtn").click();
        }
      }

      if (e.altKey && e.key == "Enter") {
        document.getElementById("html").checked = true;
        if (document.getElementById("pasteform").reportValidity()) {
          document.getElementById("pastebtn").click();
        }
      }
    });

    // Javascript enabled features
    document.getElementById("gdrive").removeAttribute("disabled");
    document.getElementById("noc-text").classList.add("d-md-block");
    document.querySelectorAll('[data-bs-toggle="tooltip"]').forEach((e) => {
      return new bootstrap.Tooltip(e);
    });

    const api_popover = new bootstrap.Popover(document.getElementById('api'), {
      trigger: 'manual',
    });
    api_popover.show();

    document.getElementById("content").focus();
  });
})();
