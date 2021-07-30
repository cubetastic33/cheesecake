// This is so that the backup selection resets to the one generated from the cookies
// The browser's cached backup selection can otherwise be inconsistent with the channel list
$("#chat_switcher").trigger("reset").on("submit", e =>  {
    e.preventDefault();
    document.cookie = "backup=" + $("#backup").val();
    document.cookie = "chat=" + $("#chat").val();
    location.href = "/reader";
});

declare var chats;

$("#backup").on("change", () => {
    // Get the active backup
    let backup = $("#backup").val() as string;
    // Empty the chat options
    let $chat = $("#chat");
    $chat.empty();
    if (!Object.keys(chats[backup]).length) {
        $chat.append("<option>No chats found</option>");
    }
    // Iterate over the chats in that backup
    for (let i = 0; i < chats[backup].length; i++) {
        let chat = chats[backup][i];
        $chat.append(`<option value="${chat[0]}">${chat[1]}</option>`);
    }
});
