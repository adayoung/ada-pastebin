"use strict";

(function () {
  window.addEventListener("DOMContentLoaded", () => {
    // Share button
    if (navigator.canShare != undefined) {
      if (navigator.canShare({ url: location.href })) {
        let shareBtn = document.getElementById("share-btn");
        shareBtn.addEventListener("click", async (e) => {
          e.preventDefault();
          try {
            await navigator.share({ url: location.href });
          } catch (error) {
            // document.getElementById('share-btn').classList.add('d-none');
          }
        });
        shareBtn.classList.remove("d-none");
      }
    }

    // Edit form modal
    if (document.getElementById("edit-paste-modal") !== null) {
      const editFormModal = new bootstrap.Modal("#edit-paste-modal");

      // Edit button
      let editBtn = document.getElementById("edit-btn");
      editBtn.classList.remove("d-none");
      editBtn.addEventListener("click", (e) => {
        e.preventDefault();

        editFormModal.show();
      });

      // Edit form
      let editForm = document.getElementById("edit-paste-form");
      let editFieldSet = document.getElementById("edit-paste-fields");
      editForm.addEventListener("submit", (e) => {
        e.preventDefault();

        let data = new FormData(e.target);
        editFieldSet.setAttribute("disabled", true);

        // Encode the form data using URLSearchParams
        const encodedData = new URLSearchParams(data);

        fetch(e.target.action, {
          method: "PATCH",
          headers: {
            "Content-Type": "application/x-www-form-urlencoded",
            "X-Requested-With": "XMLHttpRequest",
          },
          body: encodedData.toString(),
        }).then((response) => {
          if (response.ok) {
            return response.text();
          } else {
            alert(
              `Oops, we couldn't edit this paste :( The following was encountered:\n\n${response.status}: ${response.statusText}`,
            );
            throw "-flails-";
          }
        }).then((result) => {
          location.href = "/pastebin/" + result;
        }).catch((error) => {
          if (error != "-flails-") {
            alert(
              "Oops, we couldn't edit your paste :( Maybe the network pipes aren't up?",
            );
          }

          editFieldSet.removeAttribute("disabled");
        });
      });
    }

    // Fancy delete button
    document
      .getElementById("delete-form")
      .addEventListener("submit", function (e) {
        e.preventDefault();

        document.getElementById("delete-btn").setAttribute("disabled", true);
        let data = new FormData(this);

        // Encode the form data using URLSearchParams
        const encodedData = new URLSearchParams(data);

        fetch(this.getAttribute("action"), {
          method: this.getAttribute("method"),
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
              alert(
                `Oops, we couldn't delete this paste :( The following was encountered:\n\n${response.status}: ${response.statusText}`,
              );
              throw "-flails-";
            }
          })
          .then((result) => {
            alert("BAM!@ Okay! This paste is no longer available.");
            location.replace(result);
          })
          .catch((error) => {
            if (error != "-flails-") {
              alert(
                "Oops, we couldn't delete your paste :( Maybe the network pipes aren't up?",
              );
              document.getElementById("delete-btn").removeAttribute("disabled");
            }
          });
      });

    // Iframe auto-resize
    document
      .getElementById("content-frame")
      .addEventListener("load", function () {
        try {
          let height = this.contentDocument.body.scrollHeight;
          this.style.height = height + 24 + "px";
        } catch {}
      });

    let escape_html = function (input) {
      // I have no idea how this works -hides-
      let theGreateEscaper = document.createElement("p");
      theGreateEscaper.appendChild(document.createTextNode(input));
      let output = theGreateEscaper.innerHTML;
      output = output.replace(/\r\n/g, "\n");
      return output;
    };

    // Fancy content fetch
    let fetchContent = function (contentURL) {
      fetch(contentURL, {
        headers: {
          "X-Requested-With": "XMLHttpRequest",
        },
      })
        .then((response) => {
          if (response.ok) {
            return response.text();
          } else {
            document.getElementById("loader-result").textContent =
              `Meep! I couldn't get the content -flails- (${response.status}: ${response.statusText})`;
            if (document.querySelectorAll("#driveHosted").length > 0) {
              document
                .getElementById("drive-failure")
                .classList.remove("d-none");
            }
            throw "-flails-";
          }
        })
        .then(async (result) => {
          let format = document.getElementById("format").value;

          // Check if content is encrypted (look for key in URL fragment)
          let isEncrypted = false;
          let urlHash = "";
          if (document.getElementById("encrypted").value == "true") {
            isEncrypted = true;

            urlHash = window.location.hash.substring(1); // Remove # prefix
            if (!(urlHash && urlHash.length > 0)) {
              urlHash = prompt("Meep! Key not present in URL. Do you have a key?");
            }
          };

          if (isEncrypted) {
            try {
              // Decrypt the content first
              const decryptedContent = await decryptContent(result, urlHash);
              result = decryptedContent;
            } catch (e) {
              console.error('Decryption failed:', e);
              document.getElementById("loader-result").textContent =
                "Meep! I couldn't decrypt the content. Invalid or missing encryption key.";
              document.getElementById("loader").classList.remove("text-light");
              document.getElementById("loader").classList.add("text-danger");
              return;
            }
          };

          if (format == "log") {
            let output = document.getElementById("content-terminal");
            output.classList.remove("d-none");

            result = escape_html(result);
            let lines = result.split(/\r\n|\n/g);
            let txt = lines.join("<br>");

            const { AnsiUp } = await import("/static/vendor/js/ansi_up.js.br");
            let ansi_up = new AnsiUp();
            ansi_up.escape_html = false;
            output.innerHTML = ansi_up.ansi_to_html(txt);

            // Make terminal content focusable for assistive tech
            output.setAttribute("tabindex", "0");
            output.setAttribute("aria-hidden", "false");

            document.getElementById("loader-result").textContent = "Content loaded!";
            document.getElementById("loader").classList.add("d-none");
          } else if (format == "html") {
            let frame = document.getElementById("content-frame");
            frame.srcdoc = result; // This because Safari doesn't support blobs
            frame.classList.remove("d-none");

            // Make iframe focusable for assistive tech
            frame.setAttribute("tabindex", "0");
            frame.setAttribute("aria-hidden", "false");

            document.getElementById("loader-result").textContent = "Content loaded!";
            document.getElementById("loader").classList.add("d-none");
          } else {
            let textEl = document.getElementById("content-text");
            textEl.classList.remove("d-none");

            result = escape_html(result);
            textEl.innerHTML = result.replace(
              /^(.*)$/gm,
              '<span class="line">$1</span>',
            );

            // Make plain text content focusable for assistive tech
            textEl.setAttribute("tabindex", "0");
            textEl.setAttribute("aria-hidden", "false");

            document.getElementById("loader-result").textContent = "Content loaded!";
            document.getElementById("loader").classList.add("d-none");
          }
        })
        .catch((error) => {
          if (error != "-flails-") {
            console.log(error);
            document.getElementById("loader-result").textContent =
              "Meep! I couldn't get your content :( Maybe the network pipes aren't up?";
          }

          document.getElementById("loader").classList.remove("text-light");
          document.getElementById("loader").classList.add("text-danger");
        });
    };

    document.getElementById("loader").classList.remove("d-none");
    let pasteID = document.getElementById("paste-id").value;
    let contentURL = document.getElementById("content-url").value;
    if (document.querySelectorAll("#driveHosted").length > 0) {
      fetch("/pastebinc/" + pasteID + "/content/link")
        .then((response) => {
          if (response.ok) {
            return response.text();
          } else {
            document.getElementById("loader-result").textContent =
              `Meep! I couldn't get the content link -flails- (${response.status}: ${response.statusText})`;
            throw "-flails-";
          }
        })
        .then((result) => {
          fetchContent(result);
        })
        .catch((error) => {
          if (error != "-flails-") {
            document.getElementById("loader-result").textContent =
              "Meep! I couldn't get your content :( Maybe the network pipes aren't up?";
          }

          document.getElementById("loader").classList.remove("text-light");
          document.getElementById("loader").classList.add("text-danger");
        });
    } else {
      fetchContent(contentURL);
    }
  });
})();
