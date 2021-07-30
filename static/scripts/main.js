$("#chat_switcher").trigger("reset").on("submit", function (e) {
    e.preventDefault();
    document.cookie = "backup=" + $("#backup").val();
    document.cookie = "chat=" + $("#chat").val();
    location.href = "/reader";
});
$("#backup").on("change", function () {
    var backup = $("#backup").val();
    var $chat = $("#chat");
    $chat.empty();
    if (!Object.keys(chats[backup]).length) {
        $chat.append("<option>No chats found</option>");
    }
    for (var i = 0; i < chats[backup].length; i++) {
        var chat = chats[backup][i];
        $chat.append("<option value=\"" + chat[0] + "\">" + chat[1] + "</option>");
    }
});
//# sourceMappingURL=main.js.map