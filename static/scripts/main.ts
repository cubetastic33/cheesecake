declare const version;
declare const initial_chats;
declare let chats;

// Check if cheesecake is up to date
$.getJSON("https://api.github.com/repos/cubetastic33/cheesecake/releases/latest", result => {
    if (result["tag_name"] !== version) {
        $("#outdated").show();
    }
});

let $backup = $("#backup");

function show_toast(message, duration = 2000) {
    $("#toast").text(message).slideDown(200, () => {
        setTimeout(() => {
            $("#toast").slideUp(200);
        }, duration);
    });
}

function display_chats(backup) {
    $("#chat").prop("outerHTML", `<select id="chat"></select>`);
    $("label[for=\"chat\"]").text("Chat:");
    // Iterate over the chats in that backup
    for (let i = 0; i < chats[backup].length; i++) {
        let chat = chats[backup][i];
        $("#chat").append(`<option value="${chat[0]}">${chat[1]}</option>`);
    }
}

// The reset is so that the backup selection resets to the one generated from the cookies
// The browser's cached backup selection can otherwise be inconsistent with the channel list
$("#chat_switcher").trigger("reset").on("submit", e =>  {
    e.preventDefault();

    let $chat = $("#chat");

    if ($chat.is("input")) {
        // It's an encrypted backup
        if ($chat.val()) {
            document.cookie = "backup=" + $("#backup").val();
            // If the password field is not empty
            $("#chat_switcher button")
                .prop("disabled", true)
                .text("decrypting...");
            // We read $backup.val() later so disable it now to ensure it stays the same
            $backup.prop("disabled", true);

            $.post("/decrypt", {"password": $chat.val()}).done(result => {
                console.log(result);

                if (result.length) {
                    // Render the fetched chat names and reset previously decrypted chat names
                    // Updating `chats` will persist the chat names even after the backup selection
                    // is changed
                    chats = initial_chats;
                    let backup = $backup.val() as string;
                    chats[backup] = result;
                    display_chats(backup);
                    show_toast("The backup will remain decrypted until you open a different backup or quit cheesecake", 5000);
                } else {
                    show_toast("No chats found. Is your password incorrect?");
                }

                // Enable inputs
                $backup.prop("disabled", false);
                $("#chat_switcher button")
                    .text("proceed")
                    .prop("disabled", false);
            });
        }
    } else {
        document.cookie = "backup=" + $("#backup").val();
        document.cookie = "chat=" + $chat.val();
        location.href = "/reader";
    }
});

$backup.on("change", () => {
    // Get the active backup
    let backup = $backup.val() as string;

    if (!Object.keys(chats[backup]).length) {
        // It's an encrypted backup
        $("#chat").prop("outerHTML", `<input type="password" id="chat" placeholder="password">`);
        $("label[for=\"chat\"]").text("Password:");
    } else {
        display_chats(backup);
    }
});
