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
        .then((result) => {
          if (document.getElementById("format").value == "log") {
            document
              .getElementById("content-terminal")
              .classList.remove("d-none");

            // const lineCount = (result.match(/\r\n|\n/g) || []).length;
            const lines = result.split(/\r\n|\n/g);
            const longest = lines.reduce((a, b) =>
              a.length > b.length ? a : b,
            );
            const lineCount = lines.length;
            const data = result.replace(/\n/g, "\r\n");

            const term = new Terminal({
              cols: longest.length,
              rows: lineCount,
              screenReaderMode: true,
              scrollback: lineCount,
              disableStdin: true,
              wrap: true,
            });
            term.open(document.getElementById("content-terminal"));
            term.onWriteParsed(() => term.scrollToTop());
            term.write(data);

            /* Adjust the terminal columns to fit the container */
            // const containerWidth =
            //   document.getElementById("content-terminal").clientWidth;
            // const cols =
            //   Math.floor(
            //     containerWidth /
            //       term._core._renderService.dimensions.css.cell.width,
            //   ) - 1;
            const cols = 120; // this is a more sane default

            /* Account for lines that are going to be wrapped */
            let extraRows = 0;
            lines.forEach((line) => {
              extraRows += Math.ceil(line.length / cols) - 1;
            });

            term.resize(cols, lineCount + extraRows);

            document.getElementById("loader").classList.add("d-none");
          } else if (document.getElementById("format").value == "html") {
            document.getElementById("content-frame").srcdoc = result; // This because Safari doesn't support blobs
            document.getElementById("content-frame").classList.remove("d-none");
            document.getElementById("loader").classList.add("d-none");
          } else {
            document.getElementById("content-text").classList.remove("d-none");
            // I have no idea how this works -hides-
            let theGreateEscaper = document.createElement("p");
            theGreateEscaper.appendChild(document.createTextNode(result));
            result = theGreateEscaper.innerHTML;
            result = result.replace(/\r\n/g, "\n");
            document.getElementById("content-text").innerHTML = result.replace(
              /^(.*)$/gm,
              '<span class="line">$1</span>',
            );

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
